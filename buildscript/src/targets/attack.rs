nil_plugin! {
    dir = "attack",
    server = "attack",
    startcommand = "host ProtoAttack attack",
    servername = "Attack",

    fn setup_server(params) {
        params.run.link_global(
            params.root.join("nil/assets/attack.msav"),
            "attack/config/maps/attack.msav",
        );
        params.run.write("attack/config/nilConfig.toml", "");
    },
}
