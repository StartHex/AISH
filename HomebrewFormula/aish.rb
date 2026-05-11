# Homebrew formula for AISH (AI Agent Shell)
#
# Usage:
#   brew tap aish/tap
#   brew install aish
#
# Or install from local formula:
#   brew install --build-from-source ./HomebrewFormula/aish.rb

class Aish < Formula
  desc "AI Agent Shell — unified terminal manager for multiple AI coding agents"
  homepage "https://github.com/aish/aish"
  license "MIT"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/aish/aish/releases/download/v0.1.0/aish-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    else
      url "https://github.com/aish/aish/releases/download/v0.1.0/aish-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256"
    end
  end

  on_linux do
    url "https://github.com/aish/aish/releases/download/v0.1.0/aish-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "REPLACE_WITH_ACTUAL_SHA256"
  end

  depends_on "rust" => :build if build.head?

  head do
    url "https://github.com/aish/aish.git", branch: "main"
  end

  def install
    if build.head?
      system "cargo", "install", "--locked", "--root", prefix, "--path", "."
    else
      bin.install "aish"
      bin.install "aish-tui"
      bin.install "aishd"
    end

    # Shell completions
    generate_completions
  end

  def generate_completions
    # Generate shell completions when available
    ohai "Shell completions will be available in a future release"
  end

  def caveats
    <<~EOS
      AISH has been installed!

      Quick start:
        aish agent add --name local/claude --type claude
        aish exec "hello world"
        aish tui

      Daemon mode (for GUI or remote access):
        aishd &
        export AISH_SOCKET=~/.aish/daemon.sock

      Documentation: https://github.com/aish/aish
    EOS
  end

  test do
    system "#{bin}/aish", "--version"
  end
end
