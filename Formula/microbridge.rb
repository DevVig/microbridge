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
  version "0.2.0"
  license "MIT"
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/DevVig/microbridge/releases/download/v0.2.0/microbridge-v0.2.0-aarch64-apple-darwin.tar.gz"
      # sha256 filled by scripts/bump-formula.sh after each release
      sha256 "4cecb8382298e3a0f2e70ef6e69371a5f656ef156b06ef0861e9ac82036ccdab"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.2.0/microbridge-ui-v0.2.0-aarch64-apple-darwin.tar.gz"
        sha256 "92d40dcfcdfff31bfdb31dfe0ff0463e0a20ac051c6733a0253cb388eaf6cd19"
      end
    end
    on_intel do
      url "https://github.com/DevVig/microbridge/releases/download/v0.2.0/microbridge-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "30233d0e97806e19e772acc68347ba5e5909f8fd6abe8ea9fe0f32789972e069"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.2.0/microbridge-ui-v0.2.0-x86_64-apple-darwin.tar.gz"
        sha256 "cf05d452d8fdc60970b9caf512f1777e989453772897f538ed6e45aac87dc758"
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
