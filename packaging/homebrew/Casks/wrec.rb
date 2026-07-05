cask "wrec" do
  version "0.1.1"
  sha256 "33a49e8ff6e5151ea1ce7d19da0223cfbd1fab188b87c6cfbba9134a0c8cfd88" # replaced by scripts/update-homebrew.sh

  url "https://github.com/shivamhwp/wrec/releases/download/v#{version}/wrec-#{version}.dmg",
      verified: "github.com/shivamhwp/wrec/"
  name "Wrec"
  desc "The most efficient screen recorder for mac"
  homepage "https://wrec-beta.vercel.app"

  depends_on macos: ">= :sequoia"
  depends_on arch: :arm64

  app "Wrec.app"

  zap trash: [
    "~/Library/Application Support/Wrec",
    "~/.wrec",
  ]
end
