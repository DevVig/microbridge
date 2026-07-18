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
  version "0.2.2"
  license "MIT"
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/DevVig/microbridge/releases/download/v0.2.2/microbridge-v0.2.2-aarch64-apple-darwin.tar.gz"
      # sha256 filled by scripts/bump-formula.sh after each release
      sha256 "61536b90af794852236c1dfce0aaf5d5866147f7a7cbf54a910b2134cf28e0f0"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.2.2/microbridge-ui-v0.2.2-aarch64-apple-darwin.tar.gz"
        sha256 "7bf7407af75665129d370adb3d9bc5b790af4d917e0a74a2c27a68955f575b9c"
      end
    end
    on_intel do
      url "https://github.com/DevVig/microbridge/releases/download/v0.2.2/microbridge-v0.2.2-x86_64-apple-darwin.tar.gz"
      sha256 "7fe8d0187ec4915c1b6e7ceebb52f21f3db6cf572862d1b8804a15a749fdc6e6"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.2.2/microbridge-ui-v0.2.2-x86_64-apple-darwin.tar.gz"
        sha256 "90fbf26fd242ba256dded50b92bcc2ff61aa32c8dd50e3dfa3f8c2c9fccf8db5"
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
      marker="${dest}/.microbridge-brew"
      /bin/mkdir -p "${apps_dir}"
      if [ -e "${dest}" ] && [ ! -f "${marker}" ]; then
        echo "Microbridge: preserving unowned ${dest}" >&2
      else
        if [ -e "${dest}" ]; then
          /bin/rm -rf "${dest}"
        fi
        /usr/bin/ditto "${source_app}" "${dest}"
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

      Start the service once to install the marker-owned app, then open it:
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
