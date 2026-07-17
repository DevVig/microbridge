# Homebrew formula (build-from-source until a tap/bottle is published).
#
#   brew install --build-from-source ./Formula/microbridge.rb
#   brew services start microbridge
#
class Microbridge < Formula
  desc "Open-source control plane for the Codex Micro"
  homepage "https://github.com/DevVig/microbridge"
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
      Config and socket live in ~/.microbridge/
      Check the bus with: microbridgectl status
      Full install notes: #{doc}/INSTALL.md (or INSTALL.md in the repo)
    EOS
  end

  test do
    assert_match "Usage", shell_output("#{bin}/microbridgectl help")
  end
end
