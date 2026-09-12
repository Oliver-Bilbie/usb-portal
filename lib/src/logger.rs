use env_logger::Builder;
use log::LevelFilter;
use std::io::Write;

pub fn init() {
    Builder::new()
        .format(|buf, record| {
            let timestamp = chrono::Local::now().format("%H:%M:%S");
            let level_style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "{} {}{}{} {}",
                timestamp,
                level_style.render(),
                record.level(),
                level_style.render_reset(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}
