# Packaging

Templates and notes for the downstream channels that don't ship via
`cargo install` or Homebrew.

## AUR — Arch Linux

`aur-bin/PKGBUILD` is a binary package that downloads the upstream
GitHub-Release tarball for the user's architecture. Supports x86_64 and
aarch64.

### First-time publish

You need an AUR account with an SSH key registered at
<https://aur.archlinux.org/account/>.

```bash
cd /tmp
git clone ssh://aur@aur.archlinux.org/delphitools-cli-bin.git
cp /path/to/delphitools-cli/packaging/aur-bin/PKGBUILD delphitools-cli-bin/
cd delphitools-cli-bin

# Generate the .SRCINFO that AUR uses to populate its search index.
makepkg --printsrcinfo > .SRCINFO

git add PKGBUILD .SRCINFO
git commit -m "delphitools-cli-bin 0.1.0"
git push
```

### On each release

Update `pkgver`, refresh `sha256sums_x86_64` and `sha256sums_aarch64`
(values from `https://github.com/1612elphi/delphitools-cli/releases/download/vX.Y.Z/sha256.sum`),
regenerate `.SRCINFO`, push.

### Why no source package

A from-source AUR build requires either:

* network-fetching ort's prebuilt ONNX Runtime binary at build time (forbidden by AUR policy), or
* vendoring ~80 MB of upstream ORT libraries into the tarball.

Until ort 2 has a clean offline build story, the `-bin` package is the
only AUR variant we ship.

## COPR — Fedora / RHEL

`copr/delphitools-cli.spec` is a binary RPM spec that pulls the same
upstream tarballs as the AUR package. Supports x86_64 and aarch64.

### First-time publish

1. Sign in to <https://copr.fedorainfracloud.org/> with your FAS account.
2. **New Project** → name `delphitools-cli`, chroots: at least
   `fedora-rawhide-x86_64`, `fedora-rawhide-aarch64`, plus whichever
   stable Fedora versions you want to support.
3. **Builds → New Build → Upload**, attach
   `packaging/copr/delphitools-cli.spec`, and submit.
4. Users on Fedora then run:

   ```bash
   sudo dnf copr enable <fas-username>/delphitools-cli
   sudo dnf install delphitools-cli
   ```

### Automated rebuilds

COPR can re-run a build whenever you push a new tag to GitHub. Configure
under **Project Settings → Integrations → GitHub** with the
`delphitools-cli` repo and tag pattern `v*`.
