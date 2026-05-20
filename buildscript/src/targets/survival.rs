nil_plugin! {
    dir = "survival",
    server = "survival",
    startcommand = "host ProtoSurvival survival",
    servername = "Survival",

    fn setup_server(params) {
        params.run.link_global(
            params.root.join("nil/assets/survival.msav"),
            "survival/config/maps/survival.msav",
        );
        params.run.write("survival/config/nilConfig.toml", "");
    },
}
