class Dotr < Formula
  desc "A dotfiles manager that is as dear as a daughter"
  homepage "https://github.com/uroybd/DotR"
  version "0.4.4"
  
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/uroybd/DotR/releases/download/#{version}/dotr-aarch64-apple-darwin.tar.gz"
      sha256 "" # Will be auto-updated by release workflow
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/uroybd/DotR/releases/download/#{version}/dotr-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "" # Will be auto-updated by release workflow
    end
  end

  def install
    bin.install "dotr"
  end

  test do
    system "#{bin}/dotr", "--help"
  end
end
