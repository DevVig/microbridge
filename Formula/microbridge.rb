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
  version "0.3.2"
  license "MIT"
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/DevVig/microbridge/releases/download/v0.3.2/microbridge-v0.3.2-aarch64-apple-darwin.tar.gz"
      # sha256 filled by scripts/bump-formula.sh after each release
      sha256 "4266ddefd071596119fedf68bc895941453938ce1c07ef3df0a756dc0b84a53a"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.3.2/microbridge-ui-v0.3.2-aarch64-apple-darwin.tar.gz"
        sha256 "b486403a29e7bb4be012cc1fe827d1b2e6674335c16af755ad3593b9edda28e1"
      end
    end
    on_intel do
      url "https://github.com/DevVig/microbridge/releases/download/v0.3.2/microbridge-v0.3.2-x86_64-apple-darwin.tar.gz"
      sha256 "5166841bbfc05b050b04d224add95cfd837802c2cbc8599e636d2283bc7e9c16"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.3.2/microbridge-ui-v0.3.2-x86_64-apple-darwin.tar.gz"
        sha256 "08970a89e4eb7f3a7200a9ed568afcd48f1d31812d06b5dc14b56a034e0812b9"
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
