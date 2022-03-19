mod parser;

use color_eyre::eyre::Result;
use parser::def;

fn main() -> Result<()> {
    color_eyre::install()?;
    dbg!(def("#agent Add:2, Z: 1 , E :0\n#agent A:2\r\nA(c)=A(r)")?);
    Ok(())
}
