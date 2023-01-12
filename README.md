# About the project

The homotopy.io proof assistant allows the construction of composite morphisms in a finitely-generated semistrict n-category, via a point-and-click user interface. Composites are rendered as 2d and 3d geometries, and can be visualised in 4d as movies of 3d geometries. Beyond its features as a visual proof assistant, homotopy.io can also be used as an effective tool to typeset string diagrams: any 2d diagram constructed in the proof assistant can be exported with ease into LaTeX/TikZ and SVG, with experimental support for manim.

The proof assistant is implemented in the Rust programming language, and compiled to WebAssembly to run in the web browser.

For a description of how the tool works, please see the [nLab page](https://ncatlab.org/nlab/show/homotopy.io). A more recent tutorial may be found [here](./TUTORIAL.md).

The master branch is hosted live here: https://beta.homotopy.io/

# Developing

The easiest way to set up a development environment is with [Nix](https://nixos.org/).

Running

```
nix develop github:homotopy-io/homotopy-rs
```

will spawn a development shell with Rust, and all the necessary tooling required to build the project. Additionally, this is the exact same environment that the CI uses, so providing that it is passing, this cannot fail:

![Build status](https://github.com/homotopy-io/homotopy-rs/actions/workflows/ci.yml/badge.svg)

Any recent commit can be run with

```
nix run github:homotopy-io/homotopy-rs?rev=X
```

Please adhere to Rust stable and lint all code with `nix run github:homotopy-io/homotopy-rs#lint`.

## Development server

From the root of the project, run `cargo make serve`. This will build the app into `/dist`, and concurrently run a development server on http://localhost:8080, which refreshes whenever the code changes.
Alternatively, the project can be built with `nix build`, with the development server invoked by `devserver --path result/`, where `result/` is the nix-build output folder.

## Nix & GitHub Actions

This project uses GitHub Actions CI, for automating builds, testing, and deployment. GitHub Actions uses Nix to build, and the resulting compilation artifacts should like-for-like match those generated by `nix-build`. In particular, this allows binary artifacts (including witnesses of tests being run!) to be substituted across machines, in the standard way supported by Nix. Nix builds are cached on [Cachix](https://app.cachix.org/cache/homotopy-io). In other words, `nix-build | cachix push homotopy-io` allows the CI to skip compilation and test running.

Dependencies are updated via [Dependabot](https://github.com/dependabot), and are automatically merged (provided that all tests pass).
Keep the Cargo.lock updated with any changes to the various Cargo.toml, e.g. by running `cargo check` inside a dev-shell, as not doing so may cause problems with the build.

## Firebase

Builds are deployed to Firebase for hosting, by the GitHub Actions CI. https://beta.homotopy.io mirrors `master`, and any pull request generates a [Firebase Hosting preview](https://firebase.google.com/docs/hosting/test-preview-deploy#preview-channels) site, of the form homotopy-rs--homotopy-io-X.web.app.

In future, Firebase will also act as online storage for user projects.

# Citing

The tool should be cited as follows:

```
@article{homotopy-io,
  author = {Nathan Corbyn and Lukas Heidemann and Nick Hu and Calin Tataru and Jamie Vicary},
  title = {The proof assistant homotopy.io},
  url = {https://homotopy.io/},
  date = {2022},
}
```

# License

Unless explicitly stated otherwise, all contributions are licensed under the following terms.

## Source

homotopy.io source code is published under the terms of the [BSD 3-Clause License](LICENSE).

## Documentation

<a rel="license" href="http://creativecommons.org/licenses/by/4.0/"><img alt="Creative Commons License" style="border-width:0" src="https://i.creativecommons.org/l/by/4.0/88x31.png" /></a><br />homotopy.io documentation is licensed under a <a rel="license" href="http://creativecommons.org/licenses/by/4.0/">Creative Commons Attribution 4.0 International License</a>.

## Dependencies

We use the HiGHS linear programming solver for the layout algorithm:

Parallelizing the dual revised simplex method, Q. Huangfu and J. A. J. Hall, Mathematical Programming Computation, 10 (1), 119-142, 2018. DOI: 10.1007/s12532-017-0130-5
