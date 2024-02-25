use serenity::{
    all::{GatewayIntents, GuildId},
    Client,
};

use crate::config::Config;

mod config;
mod gemini;
mod riot;

/// Gets the most recent match that at least two players participated in, and ask Gemini who was
/// the worst player.
async fn get_ai_game_comment() -> String {
    let mat = match riot::get_most_recent_match().await {
        Err(err) => return format!("Failed to get most recent match: {err}"),
        Ok(None) => return "No recent match found.".to_string(),
        Ok(Some(mat)) => mat,
    };
    let judgment =
        match gemini::ask_gemini(format!(include_str!("prompt.txt"), game_data = mat)).await {
            Ok(judgment) => judgment,
            Err(err) => return format!("AI failed to analyze the result: {err}"),
        };

    format!("{}\n{}", mat.human_format(), judgment)
}

type PoiseContext<'a> = poise::Context<'a, (), anyhow::Error>;

#[poise::command(slash_command, rename = "抓戰犯")]
/// 幫你用精確無比絕不胡扯的 AI 抓出誰是上一場的雷包
///
/// The first line in the doc comment will be the description in discord.
async fn whobad(ctx: PoiseContext<'_>) -> anyhow::Result<()> {
    // Discord bots are required to reply within 5 seconds.
    // See https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-modal
    // So send something to prevent timeout.
    ctx.say("fetching...").await?;
    ctx.say(get_ai_game_comment().await).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    Config::parse("config.toml").expect("Failed to parse config");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![whobad()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                for guild_id in &Config::get().guild_ids {
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        GuildId::new(*guild_id),
                    )
                    .await?;
                }
                Ok(())
            })
        })
        .build();
    let mut client = Client::builder(&Config::get().discord_token, intents)
        .framework(framework)
        .await
        .unwrap();

    if let Err(why) = client.start().await {
        tracing::error!("Client error: {:?}", why);
    }
}
