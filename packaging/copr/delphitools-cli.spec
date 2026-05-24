Name:           delphitools-cli
Version:        0.1.0
Release:        1%{?dist}
Summary:        indie toolkit for designers — colour, image, PDF, type, calc, all in one offline CLI

License:        0BSD
URL:            https://github.com/1612elphi/delphitools-cli
# Per-arch sources fetched from the upstream GitHub Release.
Source0:        %{url}/releases/download/v%{version}/%{name}-x86_64-unknown-linux-gnu.tar.xz
Source1:        %{url}/releases/download/v%{version}/%{name}-aarch64-unknown-linux-gnu.tar.xz

ExclusiveArch:  x86_64 aarch64

# `delphi rmbg` shells out to curl to download its ML model on consent.
Requires:       curl

%description
delphitools is a self-contained CLI bundling ~40 small design and publishing
utilities — colour conversion and palette generation, image cropping/
conversion/tracing/background-removal, PDF imposition and preflight,
typographic calculators, regex testing, Unicode glyph lookup, QR/barcode
generation, scientific/unit/time calculators, Shavian transliteration, and
more. Offline by default, machine-readable with `--json`, no config files,
no telemetry.

The package installs three binary names — `delphi`, `delphitools`, `dt`
— all pointing at the same tool.

%global debug_package %{nil}

%prep
%ifarch x86_64
%setup -q -n %{name}-x86_64-unknown-linux-gnu
%endif
%ifarch aarch64
%setup -q -T -b 1 -n %{name}-aarch64-unknown-linux-gnu
%endif

%build
# Nothing to do — the upstream tarballs ship prebuilt static binaries.

%install
install -d %{buildroot}%{_bindir}
for bin in delphi delphitools dt; do
  install -m 0755 "$bin" %{buildroot}%{_bindir}/"$bin"
done

# Generate man pages directly from the installed binary.
install -d %{buildroot}%{_mandir}/man1
%{buildroot}%{_bindir}/delphi install-man --dir %{buildroot}%{_mandir}/man1

install -Dm 0644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%files
%license LICENSE
%doc README.md
%{_bindir}/delphi
%{_bindir}/delphitools
%{_bindir}/dt
%{_mandir}/man1/delphi.1*
%{_mandir}/man1/delphi-*.1*

%changelog
* Sun May 24 2026 Ruby Morgan Voigt <rmv@rmv.fyi> - 0.1.0-1
- Initial package: linux x86_64 and aarch64 binaries built upstream by
  cargo-dist, packaged here for the Fedora/EPEL ecosystem.
