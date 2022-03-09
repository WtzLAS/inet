mod machine;

mod syntax;

use color_eyre::eyre::Result;
use syntax::statement;

fn main() -> Result<()> {
    color_eyre::install()?;
    dbg!(statement("#agent Add:2, Z: 1 , E :0\n#agent A:2\r\nA(c)=A(r)")?.1);
    Ok(())
}
