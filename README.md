Track information across your repository. Ensure it never goes stale.

`kupli` is able to track information links in your repository, and can either alert you when information might be out of date, or automatically fix things if it's configured to do so.
You register links by detailing what piece of information is linked to another,
and `kupli` checks those links are still valid on every commit.

# Use cases

- Know when documentation goes out of date by linking pieces of documentation directly to the code. You can even _enforce_ documentation to be updated whenever the code changes.
- Automatically update values across your repository whenever any of the linked pieces change. Keep port numbers, addresses, and constant values updated throughout every file.

# Building

`kupli` is built with [Nix](https://nixos.org/). To build it, install Nix on your platform, enable support for flakes, and run `nix build .` on the root of this repository. The result will be symlinked at `result/bin/kupli`.
