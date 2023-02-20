use adenosine::app_bsky::PostView;
use anyhow::Result;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn pp_post_view(pv: &PostView) -> Result<()> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true))?;

    write!(&mut stdout, "@{:<54.54}", pv.author.handle)?;
    stdout.reset()?;
    stdout.set_color(ColorSpec::new().set_dimmed(true))?;
    writeln!(&mut stdout, "{}", pv.indexedAt)?;
    stdout.reset()?;

    write!(&mut stdout, " ")?;
    if let Some(entities) = &pv.record.entities {
        let mut cur: usize = 0;
        for ent in entities {
            write!(
                &mut stdout,
                "{}",
                &pv.record.text[cur..ent.index.start as usize]
            )?;
            match ent.r#type.as_str() {
                "mention" => stdout
                    .set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?,
                "hashtag" => {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Cyan)).set_bold(true))?
                }
                "link" => stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(Color::Blue))
                        .set_underline(true),
                )?,
                _ => {}
            }
            write!(
                &mut stdout,
                "{}",
                &pv.record.text[ent.index.start as usize..ent.index.end as usize]
            )?;
            stdout.reset()?;
            cur = ent.index.end as usize;
        }
        writeln!(&mut stdout, "{}", &pv.record.text[cur..])?;
    } else if !pv.record.text.is_empty() {
        writeln!(&mut stdout, "{}", &pv.record.text)?;
    }

    if let Some(embed) = &pv.embed {
        if let Some(ext) = &embed.external {
            let desc = format!("{}: {}", ext.title, ext.description);
            stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Green))
                    .set_dimmed(true)
                    .set_underline(false),
            )?;
            writeln!(&mut stdout, " {:<70.70}", desc)?;
            write!(&mut stdout, " ")?;
            stdout.set_color(
                ColorSpec::new()
                    .set_fg(Some(Color::Green))
                    .set_dimmed(true)
                    .set_underline(true),
            )?;
            writeln!(&mut stdout, "{}", &ext.uri)?;
            stdout.reset()?;
        }
        if let Some(images) = &embed.images {
            for img in images.iter() {
                if !img.alt.is_empty() {
                    stdout.set_color(
                        ColorSpec::new()
                            .set_fg(Some(Color::Green))
                            .set_dimmed(true)
                            .set_underline(false),
                    )?;
                    writeln!(&mut stdout, " {:<70.70}", img.alt)?;
                }
                write!(&mut stdout, " ")?;
                stdout.set_color(
                    ColorSpec::new()
                        .set_fg(Some(Color::Green))
                        .set_dimmed(true)
                        .set_underline(true),
                )?;
                writeln!(&mut stdout, "{}", &img.fullsize)?;
                stdout.reset()?;
            }
        }
    }

    writeln!(&mut stdout, "\n")?;
    stdout.reset()?;
    Ok(())
}
