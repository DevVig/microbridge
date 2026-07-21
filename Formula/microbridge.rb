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
  version "0.3.6"
  license "MIT"
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/DevVig/microbridge/releases/download/v0.3.6/microbridge-v0.3.6-aarch64-apple-darwin.tar.gz"
      # sha256 filled by scripts/bump-formula.sh after each release
      sha256 "812f1657df2d0ceb046d81431bf4c7a8f015dea1d7cc5d959dbf7cbfc864fd1c"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.3.6/microbridge-ui-v0.3.6-aarch64-apple-darwin.tar.gz"
        sha256 "d239cf139123751d004b9e6abfc87504358d3c84640e25c41cd96eb747129097"
      end
    end
    on_intel do
      url "https://github.com/DevVig/microbridge/releases/download/v0.3.6/microbridge-v0.3.6-x86_64-apple-darwin.tar.gz"
      sha256 "7bb5efe3aab20b3cc4720802a80bf2d631899a5993d17a9d89949966fc294c2d"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.3.6/microbridge-ui-v0.3.6-x86_64-apple-darwin.tar.gz"
        sha256 "d1d341ff29bd80b571e6d3e72c50ad1a83b433ee6262ebb8a482b73f58a7bff4"
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

    # Homebrew sandboxes formula post_install and forbids writes to $HOME.
    # The launch-agent runs in the user's session, so this wrapper performs the
    # marker-guarded app copy immediately before starting the daemon.
    service_script = libexec/"microbridge-service"
    service_script.write <<~SH
      #!/bin/sh
      set -eu
      source_app="#{opt_prefix}/Microbridge.app"
      apps_dir="${HOME}/Applications"
      dest="${apps_dir}/Microbridge.app"
      marker="${apps_dir}/.Microbridge.app.microbridge-brew"
      legacy_marker="${dest}/.microbridge-brew"
      /bin/mkdir -p "${apps_dir}"
      if [ -e "${dest}" ] && [ ! -f "${marker}" ] && [ ! -f "${legacy_marker}" ]; then
        echo "Microbridge: preserving unowned ${dest}" >&2
      else
        if [ -e "${dest}" ]; then
          /bin/rm -rf "${dest}"
        fi
        /usr/bin/ditto "${source_app}" "${dest}"
        # Keep ownership state beside the signed bundle. Adding any file to
        # Microbridge.app invalidates its sealed code signature.
        /usr/bin/touch "${marker}"
      fi
      exec "#{opt_bin}/microbridged"
    SH
    service_script.chmod 0755
  end

  service do
    run [opt_libexec/"microbridge-service"]
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

      Start the service once to install the marker-owned app, then open it. The
      app will offer to start itself at login (change it in Settings > General):
        brew services start microbridge
        open ~/Applications/Microbridge.app

      Hardware LEDs/keys need a connected Codex Micro and explicit consent in
      Microbridge Settings → Device → Enable hardware control.

      Upgrade:  brew update && brew upgrade microbridge
    EOS
  end

  test do
    assert_match "Usage", shell_output("#{bin}/microbridgectl help")
    assert_path_exists prefix/"Microbridge.app"
  end
end
