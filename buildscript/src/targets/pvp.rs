nil_plugin! {
    dir = "pvp",
    server = "pvp",
    startcommand = "host ProtoPvP pvp",
    servername = "PvP",

    fn setup_server(params) {
        params.run.link_global(
            params.root.join("nil/assets/pvp.msav"),
            "pvp/config/maps/pvp.msav",
        );
        params.run.write("pvp/config/nilConfig.toml", "");
    },
}
