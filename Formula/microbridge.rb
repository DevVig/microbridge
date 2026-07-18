# typed: false
# frozen_string_literal: true

# Homebrew formula for Microbridge — menu bar app + daemon (not CLI-only).
# Installs prebuilt GitHub Release assets (fast). Source builds: use HEAD or
# clone + ./scripts/install.sh.
#
#   brew tap DevVig/microbridge https://github.com/DevVig/microbridge
#   brew install microbridge
#   brew services start microbridge
#   open ~/Applications/Microbridge.app
#
# Upgrade:
#   brew update && brew upgrade microbridge
#
class Microbridge < Formula
  desc "Open-source control plane for the Codex Micro (menu bar + daemon)"
  homepage "https://github.com/DevVig/microbridge"
  version "0.1.0"
  license "MIT"
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/DevVig/microbridge/releases/download/v0.1.0/microbridge-v0.1.0-aarch64-apple-darwin.tar.gz"
      # sha256 filled by scripts/bump-formula.sh after each release
      sha256 "d34c3c684940d7cbca3a06168b608fe1586a1668726453aa8013fe009558c2e2"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.1.0/microbridge-ui-v0.1.0-aarch64-apple-darwin.tar.gz"
        sha256 "decbf9da3092772bef611925c31f424f650c3e74b8d220d9a27ddc74c5b2d4d8"
      end
    end
    on_intel do
      url "https://github.com/DevVig/microbridge/releases/download/v0.1.0/microbridge-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "bdc519c4157d0a199deef89f9890a77d688d6b51ec1ca0599c620fa8e28bbb5e"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.1.0/microbridge-ui-v0.1.0-x86_64-apple-darwin.tar.gz"
        sha256 "ebf0bd949b2faf261c08d33f5db5b4161fd46be8f828c55b18ed4ffd40f63ddc"
      end
    end
  end

  def install
    # Release tarball layout: microbridge-vX.Y.Z-<target>/{microbridged,microbridgectl,…}
    bin.install Dir["**/microbridged"].first
    bin.install Dir["**/microbridgectl"].first

    resource("ui").stage do
      app = Dir["**/Microbridge.app"].first
      odie "Microbridge.app missing from UI release archive" if app.nil?
      prefix.install app
    end

    # INSTALL.md ships inside the daemon archive when present.
    doc.install "INSTALL.md" if File.exist?("INSTALL.md")
  end

  def post_install
    apps = Pathname.new(Dir.home)/"Applications"
    apps.mkpath
    dest = apps/"Microbridge.app"
    marker = dest/".microbridge-brew"
    if dest.exist? && !marker.exist?
      ohai "Leaving existing ~/Applications/Microbridge.app in place (not brew-managed)"
      return
    end
    rm_r dest if dest.exist?
    cp_r prefix/"Microbridge.app", dest
    marker.write "owned-by-homebrew\n"
  end

  service do
    run [opt_bin/"microbridged"]
    keep_alive true
    log_path var/"log/microbridge.log"
    error_log_path var/"log/microbridge.log"
    environment_variables RUST_LOG: "info"
  end

  def caveats
    <<~EOS
      Microbridge is the menu bar app + a local daemon (not CLI-only).

        App:     ~/Applications/Microbridge.app
        Daemon:  brew services start microbridge
        Status:  microbridgectl status
        Config:  ~/.microbridge/

      Open the app once (or add Login Items) so the menu bar icon appears:
        open ~/Applications/Microbridge.app

      Hardware LEDs/keys need a connected Codex Micro (HID packing landing
      after device captures). Until then the UI shows Simulator / Detected.

      Upgrade:  brew update && brew upgrade microbridge
    EOS
  end

  test do
    assert_match "Usage", shell_output("#{bin}/microbridgectl help")
    assert_predicate prefix/"Microbridge.app", :exist?
  end
end
