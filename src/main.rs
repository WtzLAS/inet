mod machine;

mod syntax;

use color_eyre::eyre::Result;
use syntax::agent_def;

fn main() -> Result<()> {
    color_eyre::install()?;
    dbg!(agent_def("#agent Add:2, Z: 1 , E :0"))?;
    Ok(())
}
