{
  description = "Run commands in directories via zoxide, with optional git worktree jumping";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      manifest = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package;
    in
    {
      packages = forAllSystems (pkgs: {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = "zz-cli";
          version = manifest.version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.makeWrapper pkgs.installShellFiles ];
          postInstall = ''
            installShellCompletion --zsh completions/_zz --bash completions/zz.bash
          '';
          postFixup = ''
            wrapProgram $out/bin/zz --suffix PATH : ${pkgs.lib.makeBinPath [ pkgs.zoxide ]}
          '';
          meta = {
            description = manifest.description;
            homepage = "https://github.com/enekos/zz";
            license = pkgs.lib.licenses.mit;
            mainProgram = "zz";
          };
        };
      });
    };
}
