# typed: strict
# frozen_string_literal: true

# Homebrew formula for eka. Copy it to Formula/eka.rb in a tap repository
# (for example alexlm78/homebrew-eka) and refresh the url and checksum for each
# release with scripts/update-formula.sh (the all-zero checksum below is a
# placeholder). The version is read from the url.
class Eka < Formula
  desc "Modern replacement for ls with GNU-compatible output"
  homepage "https://github.com/alexlm78/eka"
  url "https://github.com/alexlm78/eka/archive/refs/tags/v0.4.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/alexlm78/eka.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    man1.install "docs/eka.1"
    bash_completion.install "completions/eka.bash" => "eka"
    zsh_completion.install "completions/_eka"
    fish_completion.install "completions/eka.fish"
  end

  test do
    (testpath/"a.txt").write("x")
    assert_match version.to_s, shell_output("#{bin}/eka --version")
    assert_equal "a.txt\n", shell_output("#{bin}/eka -1 #{testpath}")
  end
end
