{
	inputs = {
		flake-utils.url = "github:numtide/flake-utils";
		nixpkgs.url = "github:nixos/nixpkgs";

		rust-overlay.url = "github:oxalica/rust-overlay";
	};

	outputs = {
		self,
		nixpkgs,
		flake-utils,
		rust-overlay,
		...
	}:
		flake-utils.lib.eachDefaultSystem (system:
			let
				pkgs = import nixpkgs {
					inherit system;
					overlays = [ (import rust-overlay) ];
				};
			in with pkgs; rec {
				devShell = mkShell rec {
					buildInputs = [
						libxkbcommon
						libGL

						# WINIT_UNIX_BACKEND=wayland
						wayland

						# WINIT_UNIX_BACKEND=x11
						xorg.libXcursor
						xorg.libXrandr
						xorg.libXi
						xorg.libX11

						trunk

						#rust-bin.stable.latest.default
						(rust-bin.fromRustupToolchainFile ./rust-toolchain)
					];
					LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
				};
			});
}
