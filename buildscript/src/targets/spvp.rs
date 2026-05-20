nil_plugin! {
    dir = "spvp",
    server = "sandbox-pvp",
    startcommand = "host ProtoSPvP pvp",
    servername = "Sandbox PvP",

    fn setup_server(params) {
        params.run.link_global(
            params.root.join("nil/assets/sandbox-pvp.msav"),
            "spvp/config/maps/sandbox-pvp.msav",
        );
        params.run.write("spvp/config/nilConfig.toml", "");
    },
}
