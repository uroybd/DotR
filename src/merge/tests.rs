use std::{collections::BTreeSet, path::Path};

use toml::Table;

use super::{
    Change, Edited, Outcome, Resolution, ask, ask_reopen, check_edit, diff3, edit_in_editor,
    merge_template, span,
};

fn vars(pairs: &[(&str, &str)]) -> Table {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), toml::Value::String((*v).to_string())))
        .collect()
}

fn secrets(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_string()).collect()
}

/// Merges with every conflict answered `answer`.
fn merge_with(
    template: &str,
    deployed: &str,
    vars: &Table,
    secrets: &BTreeSet<String>,
    answer: Resolution,
) -> Outcome {
    merge_template(template, deployed.as_bytes(), vars, secrets, &mut |_| {
        Ok(answer)
    })
    .unwrap()
}

fn merge(template: &str, deployed: &str, vars: &Table) -> Outcome {
    merge_with(template, deployed, vars, &BTreeSet::new(), Resolution::Skip)
}

fn merged(outcome: Outcome) -> (String, Vec<Change>) {
    match outcome {
        Outcome::Merged { template, changes } => (template, changes),
        other => panic!("expected a merge, got {other:?}"),
    }
}

#[test]
fn unchanged_when_the_deployed_file_is_the_render() {
    let v = vars(&[("HOME", "/home/u")]);
    assert_eq!(
        merge("a = {{ HOME }}\n", "a = /home/u\n", &v),
        Outcome::Unchanged
    );
}

#[test]
fn an_edit_to_a_line_without_tags_merges_as_is() {
    let v = vars(&[("HOME", "/home/u")]);
    let (template, changes) = merged(merge(
        "home = {{ HOME }}\nshell = zsh\n",
        "home = /home/u\nshell = fish\n",
        &v,
    ));
    assert_eq!(template, "home = {{ HOME }}\nshell = fish\n");
    assert!(changes.is_empty(), "nothing was transformed: {changes:?}");
}

#[test]
fn an_added_line_holding_a_value_gets_its_tag_back() {
    let v = vars(&[("HOME", "/home/u")]);
    let (template, changes) = merged(merge(
        "home = {{ HOME }}\n",
        "home = /home/u\ndata = /home/u/Data\n",
        &v,
    ));
    assert_eq!(template, "home = {{ HOME }}\ndata = {{ HOME }}/Data\n");
    assert_eq!(
        changes,
        vec![Change::Substituted {
            line: 2,
            name: "HOME".into(),
            secret: false
        }]
    );
}

#[test]
fn an_edit_to_a_bare_variable_line_is_settled_with_the_tag_kept() {
    let v = vars(&[("HOME", "/home/u")]);
    let (template, changes) = merged(merge(
        "data = \"{{ HOME }}/Data\"\n",
        "data = \"/home/u/Documents\"\n",
        &v,
    ));
    assert_eq!(template, "data = \"{{ HOME }}/Documents\"\n");
    assert!(changes.contains(&Change::Rewritten { lines: 1..2 }));
    assert!(changes.contains(&Change::Substituted {
        line: 1,
        name: "HOME".into(),
        secret: false
    }));
}

#[test]
fn an_edit_that_drops_a_variable_is_a_conflict_not_a_silent_loss() {
    let v = vars(&[("HOME", "/home/u")]);
    let outcome = merge("data = \"{{ HOME }}/Data\"\n", "data = \"/srv/Data\"\n", &v);
    assert!(
        matches!(outcome, Outcome::Conflicts(ref c) if c.len() == 1),
        "{outcome:?}"
    );
}

#[test]
fn an_edit_to_a_filtered_tag_is_a_conflict() {
    let v = vars(&[("NAME", "dotr")]);
    let outcome = merge("name = {{ NAME | upper }}\n", "name = DOTR-X\n", &v);
    let Outcome::Conflicts(conflicts) = outcome else {
        panic!("expected a conflict, got {outcome:?}");
    };
    assert_eq!(conflicts[0].template, vec!["name = {{ NAME | upper }}"]);
    assert_eq!(conflicts[0].deployed, vec!["name = DOTR-X"]);
    assert_eq!(conflicts[0].template_lines, 1..2);
}

#[test]
fn an_edit_inside_a_loop_is_a_conflict() {
    let mut v = Table::new();
    v.insert(
        "items".into(),
        toml::Value::Array(vec!["one".into(), "two".into()]),
    );
    let template = "{% for i in items %}item {{ i }}\n{% endfor %}end\n";
    let outcome = merge(template, "item one\nitem 2\nend\n", &v);
    assert!(matches!(outcome, Outcome::Conflicts(_)), "{outcome:?}");
}

#[test]
fn edits_around_a_block_merge_and_the_block_stays() {
    let v = vars(&[("ON", "yes")]);
    let template = "first\n{% if ON %}\ninside\n{% endif %}\nlast\n";
    let base = "first\n\ninside\n\nlast\n";
    assert!(matches!(merge(template, base, &v), Outcome::Unchanged));
    let (merged_template, _) = merged(merge(template, "first!\n\ninside\n\nlast\n", &v));
    assert_eq!(
        merged_template,
        "first!\n{% if ON %}\ninside\n{% endif %}\nlast\n"
    );
}

#[test]
fn whitespace_control_round_trips() {
    let v = vars(&[("ON", "yes")]);
    let template = "a\n{%- if ON %}\nb\n{%- endif %}\nc\n";
    let base = super::compile_string(template, &v).unwrap();
    let deployed = base.replace('c', "see");
    let (merged_template, _) = merged(merge(template, &deployed, &v));
    assert_eq!(
        super::compile_string(&merged_template, &v).unwrap(),
        deployed
    );
}

#[test]
fn a_value_two_variables_hold_is_left_as_text_and_reported() {
    let v = vars(&[("A", "/same"), ("B", "/same")]);
    let (template, changes) = merged(merge("{{ A }} {{ B }}\n", "/same /same\nnew /same\n", &v));
    assert_eq!(template, "{{ A }} {{ B }}\nnew /same\n");
    assert_eq!(
        changes,
        vec![Change::Ambiguous {
            line: 2,
            names: vec!["A".into(), "B".into()]
        }]
    );
}

#[test]
fn a_short_value_is_not_written_back() {
    let v = vars(&[("X", "ab")]);
    let (template, changes) = merged(merge("x={{ X }}\n", "x=ab\ny=ab\n", &v));
    assert_eq!(template, "x={{ X }}\ny=ab\n");
    assert!(changes.is_empty());
}

#[test]
fn a_variable_the_template_does_not_use_is_not_written_back() {
    let v = vars(&[("HOME", "/home/u"), ("EDITOR", "nvim")]);
    let (template, _) = merged(merge("home={{ HOME }}\n", "home=/home/u\ned=nvim\n", &v));
    assert_eq!(template, "home={{ HOME }}\ned=nvim\n");
}

#[test]
fn a_secret_never_lands_in_the_template_as_text() {
    let v = vars(&[("HOME", "/home/u"), ("TOKEN", "abc")]);
    let (template, changes) = merged(merge_with(
        "home={{ HOME }}\n",
        "home=/home/u\ntoken=abc\n",
        &v,
        &secrets(&["TOKEN"]),
        Resolution::Skip,
    ));
    assert_eq!(template, "home={{ HOME }}\ntoken={{ TOKEN }}\n");
    assert!(!template.contains("abc"));
    assert_eq!(
        changes,
        vec![Change::Substituted {
            line: 2,
            name: "TOKEN".into(),
            secret: true
        }]
    );
}

#[test]
fn a_secret_shared_with_another_variable_is_still_written_back() {
    let v = vars(&[("TOKEN", "s3cr3t"), ("COPY", "s3cr3t")]);
    let (template, _) = merged(merge_with(
        "c={{ COPY }}\n",
        "c=s3cr3t\nt=s3cr3t\n",
        &v,
        &secrets(&["TOKEN"]),
        Resolution::Skip,
    ));
    assert!(!template.contains("s3cr3t"), "{template}");
}

#[test]
fn a_conflict_shows_secrets_masked() {
    let v = vars(&[("NAME", "dotr"), ("TOKEN", "s3cr3t")]);
    let outcome = merge_with(
        "n={{ NAME | upper }}\n",
        "n=DOTR s3cr3t\n",
        &v,
        &secrets(&["TOKEN"]),
        Resolution::Skip,
    );
    let Outcome::Conflicts(conflicts) = outcome else {
        panic!("expected a conflict, got {outcome:?}");
    };
    assert_eq!(conflicts[0].deployed, vec!["n=DOTR ***"]);
}

#[test]
fn ours_keeps_the_template_and_drops_the_deployed_change() {
    let v = vars(&[("NAME", "dotr")]);
    let (template, changes) = merged(merge_with(
        "n={{ NAME | upper }}\nkeep\nplain\n",
        "n=CHANGED\nkeep\nplain!\n",
        &v,
        &BTreeSet::new(),
        Resolution::Ours,
    ));
    assert_eq!(template, "n={{ NAME | upper }}\nkeep\nplain!\n");
    assert_eq!(
        changes,
        vec![Change::KeptTemplate {
            lines: 1..2,
            deployed_lines: 1..2
        }]
    );
}

#[test]
fn theirs_takes_the_deployed_lines_with_tags_put_back() {
    let v = vars(&[("NAME", "dotr"), ("HOME", "/home/u")]);
    let (template, changes) = merged(merge_with(
        "n={{ NAME | upper }}\nh={{ HOME }}\n",
        "n=Custom /home/u\nh=/home/u\n",
        &v,
        &BTreeSet::new(),
        Resolution::Theirs,
    ));
    assert_eq!(template, "n=Custom {{ HOME }}\nh={{ HOME }}\n");
    assert!(changes.contains(&Change::TookDeployed { lines: 1..2 }));
}

#[test]
fn both_keeps_the_template_and_adds_the_deployed_lines_after() {
    let v = vars(&[("NAME", "dotr")]);
    let (template, changes) = merged(merge_with(
        "n={{ NAME | upper }}\nend\n",
        "n=CHANGED\nend\n",
        &v,
        &BTreeSet::new(),
        Resolution::Both,
    ));
    assert_eq!(template, "n={{ NAME | upper }}\nn=CHANGED\nend\n");
    assert_eq!(
        changes,
        vec![Change::Union {
            lines: 1..2,
            added: 2..3
        }]
    );
}

#[test]
fn both_at_the_end_of_a_file_without_a_newline() {
    let v = vars(&[("NAME", "dotr")]);
    let (template, _) = merged(merge_with(
        "n={{ NAME | upper }}",
        "n=CHANGED",
        &v,
        &BTreeSet::new(),
        Resolution::Both,
    ));
    assert_eq!(template, "n={{ NAME | upper }}\nn=CHANGED");
}

#[test]
fn skip_asks_no_more_and_reports_every_conflict() {
    let v = vars(&[("A", "aaaa"), ("B", "bbbb")]);
    let mut asked = 0;
    let outcome = merge_template(
        "{{ A | upper }}\nmiddle\n{{ B | upper }}\n",
        b"x\nmiddle\ny\n",
        &v,
        &BTreeSet::new(),
        &mut |_| {
            asked += 1;
            Ok(Resolution::Skip)
        },
    )
    .unwrap();
    assert_eq!(asked, 1);
    assert!(
        matches!(outcome, Outcome::Conflicts(ref c) if c.len() == 2),
        "{outcome:?}"
    );
}

#[test]
fn tag_syntax_in_the_deployed_file_fails_the_round_trip_and_writes_nothing() {
    let v = vars(&[("HOME", "/home/u")]);
    let outcome = merge("h={{ HOME }}\n", "h=/home/u\nraw={{ nope }}\n", &v);
    assert!(
        matches!(outcome, Outcome::RoundTripFailed { error: Some(_) }),
        "{outcome:?}"
    );
}

#[test]
fn a_binary_deployed_file_is_not_merged() {
    let v = vars(&[("HOME", "/home/u")]);
    let binary = [0x89, b'P', b'N', b'G', 0x00, 0xff, 0xfe];
    let outcome = merge_template("h={{ HOME }}\n", &binary, &v, &BTreeSet::new(), &mut |_| {
        Ok(Resolution::Theirs)
    })
    .unwrap();
    assert_eq!(outcome, Outcome::NotText);
    let with_nul = merge_template(
        "h={{ HOME }}\n",
        b"h=\0\n",
        &v,
        &BTreeSet::new(),
        &mut |_| Ok(Resolution::Theirs),
    )
    .unwrap();
    assert_eq!(with_nul, Outcome::NotText);
}

#[test]
fn empty_files_do_not_trip_the_merge() {
    let v = vars(&[("HOME", "/home/u")]);
    let (template, _) = merged(merge_with(
        "h={{ HOME }}\n",
        "",
        &v,
        &BTreeSet::new(),
        Resolution::Theirs,
    ));
    assert_eq!(template, "");
    // Text added where the template has only a comment is a conflict (both sides wrote there); "both" keeps each.
    assert!(matches!(
        merge("{# only a comment #}", "new\n", &v),
        Outcome::Conflicts(_)
    ));
    let (template, _) = merged(merge_with(
        "{# only a comment #}",
        "new\n",
        &v,
        &BTreeSet::new(),
        Resolution::Both,
    ));
    assert_eq!(template, "{# only a comment #}\nnew\n");
}

#[test]
fn diff3_covers_every_line_of_all_three() {
    let cases: [(&[&str], &[&str], &[&str]); 5] = [
        (&[], &[], &[]),
        (&["a"], &[], &["b"]),
        (&["a", "b", "c"], &["a", "x", "c"], &["a", "b", "y", "c"]),
        (&["x"], &["y", "x", "z"], &["x", "x"]),
        (&["a", "b"], &["b", "a"], &["a", "a", "b", "b"]),
    ];
    for (base, ours, theirs) in cases {
        let chunks = diff3(base, ours, theirs);
        let total = |f: fn(&super::Chunk) -> usize| chunks.iter().map(f).sum::<usize>();
        assert_eq!(total(|c| c.base.len()), base.len());
        assert_eq!(total(|c| c.ours.len()), ours.len());
        assert_eq!(total(|c| c.theirs.len()), theirs.len());
    }
}

#[test]
fn ask_repeats_until_it_understands_and_skips_at_end_of_input() {
    let conflict = super::Conflict {
        template_lines: 1..2,
        template: vec!["n={{ NAME | upper }}".into()],
        deployed_lines: 1..2,
        deployed: vec!["n=X".into()],
    };
    let mut out = vec![];
    let answer = ask(Path::new("f"), &conflict, &mut &b"what\nT\n"[..], &mut out).unwrap();
    assert_eq!(answer, Resolution::Theirs);
    let shown = String::from_utf8(out).unwrap();
    assert!(
        shown.contains("template line 1:") && shown.contains("    1 | n=X"),
        "{shown}"
    );
    assert_eq!(shown.matches("[o]urs").count(), 2);
    let answer = ask(Path::new("f"), &conflict, &mut &b"e\n"[..], &mut vec![]).unwrap();
    assert_eq!(answer, Resolution::Edit);

    let answer = ask(Path::new("f"), &conflict, &mut &b""[..], &mut vec![]).unwrap();
    assert_eq!(answer, Resolution::Skip);
}

#[test]
fn spans_read_naturally() {
    assert_eq!(span(&(3..4)), "line 3");
    assert_eq!(span(&(3..6)), "lines 3-5");
    assert_eq!(span(&(3..3)), "(no lines, before line 3)");
}

#[test]
fn edit_marks_this_conflict_and_every_later_one_without_asking_again() {
    let v = vars(&[("A", "aaaa"), ("TOKEN", "s3cr3t")]);
    let mut asked = 0;
    let outcome = merge_template(
        "{{ A | upper }}\nmiddle\n{{ A | lower }}\n",
        b"x s3cr3t\nmiddle!\ny\n",
        &v,
        &secrets(&["TOKEN"]),
        &mut |_| {
            asked += 1;
            Ok(Resolution::Edit)
        },
    )
    .unwrap();
    assert_eq!(asked, 1);
    let Outcome::Edit { draft, .. } = outcome else {
        panic!("expected a draft, got {outcome:?}");
    };
    assert_eq!(draft.matches("<<<<<<< template").count(), 1, "{draft}");
    assert!(
        draft.contains("x {{ TOKEN }}\n"),
        "the secret goes in as a tag: {draft}"
    );
    assert!(!draft.contains("s3cr3t"));
    assert!(draft.contains("=======\n"));
    assert!(draft.contains(">>>>>>> deployed, lines 1-3"));
}

#[test]
fn a_saved_edit_is_refused_while_markers_are_left() {
    let v = vars(&[]);
    let err = check_edit(
        "a\n<<<<<<< template, line 1\nb\n",
        "a\n",
        &v,
        &BTreeSet::new(),
    );
    assert_eq!(err, Err("line 2 still has a conflict marker".into()));
    assert!(check_edit("a\n=======\n", "a\n", &v, &BTreeSet::new()).is_err());
}

#[test]
fn a_saved_edit_is_refused_when_it_holds_a_secret_as_text() {
    let v = vars(&[("TOKEN", "s3cr3t")]);
    let err = check_edit("t=s3cr3t\n", "t=s3cr3t\n", &v, &secrets(&["TOKEN"])).unwrap_err();
    assert!(
        err.contains("`TOKEN`") && err.contains("{{ TOKEN }}"),
        "{err}"
    );
    assert_eq!(
        check_edit("t={{ TOKEN }}\n", "t=s3cr3t\n", &v, &secrets(&["TOKEN"])),
        Ok(Edited {
            missing: 0,
            added: 0
        })
    );
}

#[test]
fn a_saved_edit_that_does_not_compile_is_refused() {
    let err = check_edit("{% if %}\n", "", &vars(&[]), &BTreeSet::new()).unwrap_err();
    assert!(err.starts_with("it does not compile"), "{err}");
}

#[test]
fn a_saved_edit_says_how_far_its_render_is_from_the_deployed_file() {
    let edited = check_edit(
        "one\ntwo\n",
        "one\nthree\nfour\n",
        &vars(&[]),
        &BTreeSet::new(),
    )
    .unwrap();
    assert_eq!(
        edited,
        Edited {
            missing: 2,
            added: 1
        }
    );
    assert!(!edited.renders_deployed());
}

#[cfg(unix)]
#[test]
fn the_editor_round_trip_returns_what_was_saved_and_reports_a_failed_editor() {
    let saved = edit_in_editor("true", "draft\n", Path::new("dotfiles/env.nu")).unwrap();
    assert_eq!(saved, "draft\n");
    let err = edit_in_editor("false", "draft\n", Path::new("x")).unwrap_err();
    assert!(err.to_string().contains("exited"), "{err}");
    assert!(edit_in_editor("  ", "d", Path::new("x")).is_err());
}

#[test]
fn a_template_that_does_not_render_is_not_merged() {
    let outcome = merge_template(
        "t = {{ NEVER_DEFINED }}\n",
        b"t = x\n",
        &toml::Table::new(),
        &BTreeSet::new(),
        &mut |_| Ok(Resolution::Skip),
    )
    .unwrap();
    let Outcome::NotRendered { error } = outcome else {
        panic!("expected NotRendered, got {outcome:?}");
    };
    assert!(error.contains("NEVER_DEFINED"), "{error}");
}

#[test]
fn ask_reopen_reopens_only_when_asked() {
    for (answer, reopen) in [
        ("r\n", true),
        ("Reopen\n", true),
        ("s\n", false),
        ("x\n", false),
        ("", false),
    ] {
        let mut out = vec![];
        assert_eq!(
            ask_reopen(&mut answer.as_bytes(), &mut out).unwrap(),
            reopen,
            "{answer:?}"
        );
    }
}

#[test]
fn a_file_too_big_to_line_up_by_pattern_still_merges_by_exact_lines() {
    // 2,001 lines against 2,001 is past MAX_PATTERN_PAIRS: the template is lined up with its render by exact matches
    // only, and an edit to a plain line still merges, the tag line kept.
    let mut template = String::from("name = {{ NAME }}\n");
    for i in 0..2000 {
        template.push_str(&format!("line {i}\n"));
    }
    let deployed = template
        .replace("{{ NAME }}", "dotr-user")
        .replace("line 1500\n", "line 1500 changed\n");

    let outcome = merge_template(
        &template,
        deployed.as_bytes(),
        &vars(&[("NAME", "dotr-user")]),
        &BTreeSet::new(),
        &mut |_| Ok(Resolution::Skip),
    )
    .unwrap();

    let Outcome::Merged {
        template: merged, ..
    } = outcome
    else {
        panic!("expected a merge, got {outcome:?}");
    };
    assert!(merged.starts_with("name = {{ NAME }}\n"));
    assert!(merged.contains("line 1500 changed\n"));
}
