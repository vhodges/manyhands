{ lib, rustPlatform, fetchCrate, pkg-config, zstd }:
rustPlatform.buildRustPackage {
  pname = "cargo-about";
  version = "0.9.3";
  src = fetchCrate {
    pname = "cargo-about";
    version = "0.9.3";
    unpack = false;
    hash = "sha256-8jCeTnzW+7JcTI1o5pxyiTbexYoMt5QNyzgtW+MqpSs=";
  };
  cargoHash = "sha256-kNGfQQumgvtDPhiQcqc9K2ohhEsTzuySeisHcNcmig8=";
  buildFeatures = [ "cli" ];
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ zstd ];
  env.ZSTD_SYS_USE_PKG_CONFIG = true;
  doCheck = false;
  meta = {
    description = "Pinned offline notice generator for Manyhands";
    homepage = "https://github.com/EmbarkStudios/cargo-about";
    license = with lib.licenses; [ mit asl20 ];
    mainProgram = "cargo-about";
  };
}
