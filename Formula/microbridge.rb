# typed: false
# frozen_string_literal: true

# Homebrew formula for Microbridge — menu bar app + daemon (not CLI-only).
# Installs prebuilt GitHub Release assets (fast). Source builds: use HEAD or
# clone + ./scripts/install.sh.
#
#   brew tap DevVig/microbridge https://github.com/DevVig/microbridge
#   brew install microbridge
#   microbridge-app install
#
# Upgrade:
#   brew update && brew upgrade microbridge
#
class Microbridge < Formula
  desc "Open-source control plane for the Codex Micro (menu bar + daemon)"
  homepage "https://github.com/DevVig/microbridge"
  version "0.3.9"
  license "MIT"
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on :macos

  on_macos do
    on_arm do
      url "https://github.com/DevVig/microbridge/releases/download/v0.3.9/microbridge-v0.3.9-aarch64-apple-darwin.tar.gz"
      # sha256 filled by scripts/bump-formula.sh after each release
      sha256 "7d4671e8448c421b64afc297ef7b7bf11d86e8f82b66a66a85504f0d4ec8bfb2"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.3.9/microbridge-ui-v0.3.9-aarch64-apple-darwin.tar.gz"
        sha256 "7d4c52e27eada6995bb9f8cd2833fff6e31c0e6886db4d7f608ab2c8544399e0"
      end
    end
    on_intel do
      url "https://github.com/DevVig/microbridge/releases/download/v0.3.9/microbridge-v0.3.9-x86_64-apple-darwin.tar.gz"
      sha256 "131a7e4d76949fc65180345834fb915f7c5a461ad0b1aa062f8c2afacc27d833"

      resource "ui" do
        url "https://github.com/DevVig/microbridge/releases/download/v0.3.9/microbridge-ui-v0.3.9-x86_64-apple-darwin.tar.gz"
        sha256 "5b95f275b13bee41f7b73b3b4481753f0703b25c643709abed6a54dae4af842d"
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

    # Homebrew sandboxes formula installation from $HOME. This explicit helper
    # performs the marker-guarded GUI install without registering a daemon
    # service; `brew services` remains available for deliberate headless use.
    app_installer = bin/"microbridge-app"
    app_installer.write <<~SH
      #!/bin/sh
      set -eu
      source_app="#{opt_prefix}/Microbridge.app"
      apps_dir="${HOME}/Applications"
      dest="${apps_dir}/Microbridge.app"
      marker="${apps_dir}/.Microbridge.app.microbridge-brew"
      legacy_marker="${dest}/.microbridge-brew"
      stop_managed_app() {
        executable="${dest}/Contents/MacOS/microbridge-ui"
        /usr/bin/pgrep -f "^${executable}$" 2>/dev/null | while read -r pid; do
          /bin/kill "${pid}" 2>/dev/null || true
        done
        for _ in 1 2 3 4 5 6 7 8 9 10; do
          /usr/bin/pgrep -f "^${executable}$" >/dev/null 2>&1 || return 0
          /bin/sleep 0.1
        done
      }
      action="${1:-install}"
      if [ "${action}" = "uninstall" ]; then
        if [ -f "${marker}" ] || [ -f "${legacy_marker}" ]; then
          if [ -x "${dest}/Contents/MacOS/microbridge-ui" ]; then
            "${dest}/Contents/MacOS/microbridge-ui" --unregister-login-item || true
          fi
          stop_managed_app
          /bin/rm -rf "${dest}"
          /bin/rm -f "${marker}"
        else
          echo "Microbridge: preserving unowned ${dest}" >&2
        fi
        exit 0
      fi
      if [ "${action}" != "install" ]; then
        echo "usage: microbridge-app [install|uninstall]" >&2
        exit 2
      fi
      /bin/mkdir -p "${apps_dir}"
      if [ -e "${dest}" ] && [ ! -f "${marker}" ] && [ ! -f "${legacy_marker}" ]; then
        echo "Microbridge: preserving unowned ${dest}" >&2
        exit 1
      else
        staging="${apps_dir}/.Microbridge.app.installing.$$"
        trap '/bin/rm -rf "${staging}"' EXIT
        /bin/rm -rf "${staging}"
        /usr/bin/ditto "${source_app}" "${staging}"
        /usr/bin/codesign --verify --deep --strict "${staging}"
        if [ -e "${dest}" ]; then
          stop_managed_app
          /bin/rm -rf "${dest}"
        fi
        /bin/mv "${staging}" "${dest}"
        # Keep ownership state beside the signed bundle. Adding any file to
        # Microbridge.app invalidates its sealed code signature.
        /usr/bin/touch "${marker}"
        /usr/bin/open "${dest}"
      fi
    SH
    app_installer.chmod 0755
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
        Daemon:  app-owned (standard) or brew services (headless)
        Status:  microbridgectl status
        Config:  ~/.microbridge/

      Install or refresh the marker-owned app, then let it own the bundled daemon:
        microbridge-app install

      The app will offer to start itself at login (change it in Settings > General).

      Optional headless daemon service (this creates a separate background item):
        brew services start microbridge

      Hardware LEDs/keys need a connected Codex Micro and explicit consent in
      Microbridge Settings → Device → Enable hardware control.

      Upgrade:  brew update && brew upgrade microbridge && microbridge-app install
    EOS
  end

  test do
    assert_match "Usage", shell_output("#{bin}/microbridgectl help")
    assert_path_exists prefix/"Microbridge.app"
    assert_path_exists bin/"microbridge-app"
  end
end
