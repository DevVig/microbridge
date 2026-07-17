# typed: false
# frozen_string_literal: true

# Homebrew formula for Microbridge — menu bar app + daemon (not CLI-only).
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
  url "https://github.com/DevVig/microbridge/archive/refs/tags/v0.0.1.tar.gz"
  sha256 "f171c275890add016045b0bbde54330f104b6d5db3a9d16c8d366cd5fcdde599"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on "rust" => :build
  depends_on "node" => :build
  depends_on :macos

  def install
    system "cargo", "build", "--release", "--locked", "-p", "microbridged", "-p", "microbridgectl"
    bin.install "target/release/microbridged"
    bin.install "target/release/microbridgectl"

    cd "apps/microbridge-ui" do
      system "npm", "ci"
      system "npm", "run", "tauri", "build", "--", "--bundles", "app"
    end

    app = Dir["apps/microbridge-ui/src-tauri/target/release/bundle/macos/*.app"].first
    odie "Microbridge.app missing after Tauri build" if app.nil?
    prefix.install app

    doc.install "INSTALL.md" if File.exist?("INSTALL.md")
  end

  def post_install
    apps = Pathname.new(Dir.home)/"Applications"
    apps.mkpath
    dest = apps/"Microbridge.app"
    # Only replace if missing or previously installed by this formula.
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

      Upgrade:  brew update && brew upgrade microbridge
    EOS
  end

  test do
    assert_match "Usage", shell_output("#{bin}/microbridgectl help")
    assert_predicate prefix/"Microbridge.app", :exist?
  end
end
