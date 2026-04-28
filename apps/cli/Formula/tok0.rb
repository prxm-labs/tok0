class Tok0 < Formula
  desc "Token-optimized CLI proxy — 60-90% savings on LLM dev operations"
  homepage "https://github.com/prxm-labs/tok0"
  version "0.1.0"

  # SHA-256 checksums are updated automatically by scripts/update-formula.sh
  # after each GitHub Release. Do not edit manually.
  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/prxm-labs/tok0/releases/download/v#{version}/tok0-aarch64-apple-darwin"
      sha256 "d4468177d743753772f129b01a025b87df78c3c60c73dc9719691b80aea0dc5b"
    else
      url "https://github.com/prxm-labs/tok0/releases/download/v#{version}/tok0-x86_64-apple-darwin"
      sha256 "e882dadeaebcac1af67c5045249efb48a0a0afa74b80a473edac6a6fe1533e6b"
    end
  end

  def install
    binary_name = Hardware::CPU.arm? ? "tok0-aarch64-apple-darwin" : "tok0-x86_64-apple-darwin"
    bin.install binary_name => "tok0"
  end

  def caveats
    <<~EOS
      Run `tok0 init` to install hooks into your AI tools.
      Run `tok0 stats` to see your token savings.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tok0 --version")
  end
end
