# typed: false
# frozen_string_literal: true

# Homebrew formula for Microbridge (source build — fast enough for alpha).
#
#   brew tap DevVig/microbridge https://github.com/DevVig/microbridge
#   brew install microbridge
#   brew services start microbridge
#
# Upgrade (auto-update path):
#   brew update && brew upgrade microbridge
#   brew autoupdate start --upgrade --cleanup   # optional background updates
#
class Microbridge < Formula
  desc "Open-source control plane for the Codex Micro"
  homepage "https://github.com/DevVig/microbridge"
  url "https://github.com/DevVig/microbridge/archive/refs/tags/v0.0.1.tar.gz"
  sha256 "f171c275890add016045b0bbde54330f104b6d5db3a9d16c8d366cd5fcdde599"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/DevVig/microbridge.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "build", "--release", "--locked", "-p", "microbridged", "-p", "microbridgectl"
    bin.install "target/release/microbridged"
    bin.install "target/release/microbridgectl"
    doc.install "INSTALL.md" if File.exist?("INSTALL.md")
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
      Config and socket: ~/.microbridge/
      Status:            microbridgectl status
      Upgrade:           brew update && brew upgrade microbridge
      Background updates: brew autoupdate start --upgrade --cleanup
    EOS
  end

  test do
    assert_match "Usage", shell_output("#{bin}/microbridgectl help")
  end
end
