name := 'cosmic-desk-log'
appid := 'io.github.sbj_ee.CosmicDeskLog'

build:
    cargo build --release

install: build
    install -Dm0755 target/release/{{name}} ~/.local/bin/{{name}}
    install -Dm0644 data/{{appid}}.desktop ~/.local/share/applications/{{appid}}.desktop
    install -Dm0644 systemd/{{name}}.service ~/.config/systemd/user/{{name}}.service
    systemctl --user daemon-reload

uninstall:
    systemctl --user disable --now {{name}}.service || true
    rm -f ~/.local/bin/{{name}}
    rm -f ~/.local/share/applications/{{appid}}.desktop
    rm -f ~/.config/systemd/user/{{name}}.service
    systemctl --user daemon-reload

enable: install
    systemctl --user enable --now {{name}}.service
