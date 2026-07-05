class WrecCli < Formula
  desc "Efficient screen recorder CLI for terminals, scripts, and agents"
  homepage "https://wrec.app"
  url "https://github.com/shivamhwp/wrec/releases/download/v0.1.1/wrec-cli-aarch64-apple-darwin.tar.gz"
  sha256 "bd5f297ed722797c4453e1351cc6af8f434ecd9a1eaa82ea3078d1975e854174" # replaced by scripts/update-homebrew.sh
  license "MIT"

  depends_on arch: :arm64
  depends_on :macos

  def install
    libexec.install "wrec", "daemon", "capture-engine"
    bin.write_exec_script libexec/"wrec"
  end

  test do
    system bin/"wrec", "-V"
  end
end
