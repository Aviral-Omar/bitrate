name := `grep -m 1 -oP '(?<=<binary>).*?(?=</binary>)' $(ls ./res/*.xml | head -n 1)`
architecture := if arch() == "x86_64" { "amd64" } else { arch() }
version := `sed -En 's/version[[:space:]]*=[[:space:]]*"([^"]+)"/\1/p' Cargo.toml | head -1`

debname := name+'_'+version+'_'+architecture
debdir := debname / 'DEBIAN'
debcontrol := debdir / 'control'

rpmarch := arch()
rpmname := name + '-' + version + '-1.' + rpmarch
rpmdir := rpmname / 'BUILDROOT'
rpminstall := rpmdir / prefix
rpm_bin_dst := rpminstall / 'bin' / name
rpm_desktop_dst := rpminstall / 'share' / 'applications' / desktop
rpm_metainfo_dst := rpminstall / 'share' / 'metainfo' / metainfo
rpm_icons_dst := rpminstall / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps'

appid := `grep -m 1 -oP '(?<=<id>).*?(?=</id>)' $(ls ./res/*.xml | head -n 1)`
summary := `grep -m 1 -oP '(?<=<summary>).*?(?=</summary>)' $(ls ./res/*.xml | head -n 1)`
dev_name := `grep -m 1 -zoP '(?s)<developer.*?>\s*<name>\K.*?(?=</name>)' $(ls ./res/*.xml | head -n 1)`
email := `grep -m 1 -oP '(?<=<update_contact>).*?(?=</update_contact>)' $(ls ./res/*.xml | head -n 1)`

rootdir := ''
prefix := '/usr'
flatpak-prefix := '/app'

# Installation paths
base-dir := absolute_path(clean(rootdir / prefix))
flatpak-base-dir := absolute_path(clean(rootdir / flatpak-prefix))

bin-src := 'target' / 'release' / name
bin-dst := base-dir / 'bin' / name
flatpak-bin-dst := flatpak-base-dir / 'bin' / name

desktop := appid + '.desktop'
desktop-src := 'res' / desktop
desktop-dst := base-dir / 'share' / 'applications' / desktop
flatpak-desktop-dst := flatpak-base-dir / 'share' / 'applications' / desktop

metainfo := appid + '.metainfo.xml'
metainfo-src := 'res' / metainfo
metainfo-dst := base-dir / 'share' / 'metainfo' / metainfo
flatpak-metainfo-dst := flatpak-base-dir / 'share' / 'metainfo' / metainfo

icons-src := 'res' / 'icons'
icons-dst := base-dir / 'share' / 'icons' / 'hicolor' / 'scalable'
flatpak-icons-dst := flatpak-base-dir / 'share' / 'icons' / 'hicolor' / 'scalable'

# Default recipe which runs `just build-release`
default: build-release

# Runs `cargo clean`
clean:
    cargo clean

# Removes vendored dependencies
clean-vendor:
    rm -rf .cargo vendor vendor.tar

# `cargo clean` and removes vendored dependencies
clean-dist: clean clean-vendor

# Compiles with debug profile
build-debug *args:
    cargo build {{args}}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compiles release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Runs a clippy check
check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

# Runs a clippy check with JSON message format
check-json: (check '--message-format=json')

dev *args:
    cargo fmt
    just run {{args}}

# Run the application for testing purposes
run *args:
    env RUST_BACKTRACE=full cargo run --release {{args}}

# Installs files
install:
    strip {{bin-src}}
    install -Dm0755 {{ bin-src }} {{bin-dst}}
    install -Dm0644 {{desktop-src}} {{desktop-dst}}
    install -Dm0644 {{metainfo-src}} {{metainfo-dst}}
    for svg in {{icons-src}}/apps/*.svg; do \
        install -D "$svg" "{{icons-dst}}/apps/$(basename $svg)"; \
    done

# Uninstalls installed files
uninstall:
    rm {{bin-dst}} {{desktop-dst}} {{metainfo-dst}}
    for svg in {{icons-src}}/apps/*.svg; do \
        rm "{{icons-dst}}/apps/$(basename $svg)"; \
    done

# Vendor dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor --sync Cargo.toml | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    echo >> .cargo/config.toml
    rm -rf .cargo vendor

# Extracts vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Bump cargo version, create git commit, and create tag
tag version:
    #!/usr/bin/env bash
    find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{version}}"/' '{}' \; -exec git add '{}' \;
    cargo check
    cargo clean
    git add Cargo.lock
    for svg in {{icons-src}}/apps/*.svg;
    do
        rm "{{icons-dst}}/apps/$(basename $svg)"
    done
    git commit -m 'release: {{version}}'
    git tag -a {{version}} -m ''


deb:
    #!/usr/bin/env bash
    strip {{bin-src}}
    install -D {{bin-src}} {{debname}}{{bin-dst}}
    install -D {{desktop-src}} {{debname}}{{desktop-dst}}
    for svg in {{icons-src}}/apps/*.svg; do \
        install -D "$svg" "{{debname}}{{icons-dst}}/apps/$(basename $svg)"; \
    done
    mkdir -p {{debdir}}
    echo "Package: {{name}}" > {{debcontrol}}
    echo "Version: {{version}}" >> {{debcontrol}}
    echo "Architecture: {{architecture}}" >> {{debcontrol}}
    echo "Maintainer: {{dev_name}} <{{email}}>" >> {{debcontrol}}
    echo "Description: {{summary}}" >> {{debcontrol}}
    dpkg-deb --build --root-owner-group {{debname}}
    rm -Rf {{debname}}/

rpm:
    #!/usr/bin/env bash
    strip {{bin-src}}
    install -D {{bin-src}} {{rpm_bin_dst}}
    install -D {{desktop-src}} {{rpm_desktop_dst}}
    install -D {{metainfo-src}} {{rpm_metainfo_dst}}
    for svg in {{icons-src}}/apps/*.svg; do \
        install -D "$svg" "{{rpm_icons_dst}}/$(basename $svg)"; \
    done

    mkdir -p {{rpmname}}
    echo "Name: {{name}}" > {{rpmname}}/spec.spec
    echo "Version: {{version}}" >> {{rpmname}}/spec.spec
    echo "Release: 1%{?dist}" >> {{rpmname}}/spec.spec
    echo "Summary: {{summary}}" >> {{rpmname}}/spec.spec
    echo "" >> {{rpmname}}/spec.spec
    echo "License: GPLv3" >> {{rpmname}}/spec.spec
    echo "Group: Applications/Utilities" >> {{rpmname}}/spec.spec
    echo "%description" >> {{rpmname}}/spec.spec
    echo "{{summary}}" >> {{rpmname}}/spec.spec
    echo "" >> {{rpmname}}/spec.spec
    echo "%files" >> {{rpmname}}/spec.spec
    echo "%defattr(-,root,root,-)" >> {{rpmname}}/spec.spec
    echo "{{prefix}}/bin/{{name}}" >> {{rpmname}}/spec.spec
    echo "{{prefix}}/share/applications/{{desktop}}" >> {{rpmname}}/spec.spec
    echo "{{prefix}}/share/metainfo/{{metainfo}}" >> {{rpmname}}/spec.spec
    echo "{{prefix}}/share/icons/hicolor/scalable/apps/*.svg" >> {{rpmname}}/spec.spec

    rpmbuild -bb --buildroot="$(pwd)/{{rpmdir}}" {{rpmname}}/spec.spec \
        --define "_rpmdir $(pwd)" \
        --define "_topdir $(pwd)/{{rpmname}}" \
        --define "_buildrootdir $(pwd)/{{rpmdir}}"

    rm -rf {{rpmname}} {{rpmdir}}
    mv x86_64/* .
    rmdir x86_64

# Build and install flatpak locally
flatpak-install:
    #!/usr/bin/env bash
    set -e
    arch="$(flatpak --default-arch)"
    set -x
    flatpak-builder \
        --arch="${arch}" \
        --ccache \
        --force-clean \
        --install \
        --install-deps-from=flathub \
        --repo=repo \
        --user \
        "flatpak-out/${arch}" \
        {{appid}}.json

# Build flatpak locally
flatpak-build:
    #!/usr/bin/env bash
    set -e
    arch="$(flatpak --default-arch)"
    set -x
    flatpak-builder \
        --arch="${arch}" \
        --ccache \
        --force-clean \
        --install-deps-from=flathub \
        --repo=repo \
        --sandbox \
        --user \
        --verbose \
        "flatpak-out/${arch}" \
        "{{appid}}.json" \
        2>&1 | tee "log/${arch}.txt"

# Update flatpak cargo-sources.json
flatpak-cargo-sources:
    python3 ./flatpak/flatpak-cargo-generator.py ./Cargo.lock -o ./flatpak/cargo-sources.json
