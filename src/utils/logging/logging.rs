
use std::{io::{self, Write}, sync::LazyLock};

use tokio::sync::Mutex;

// Define una estructura que contiene dos destinos de escritura
pub struct LogWriter<W1 : Write, W2 : Write > {
    pub writer1: W1,
    pub writer2: W2,
}


impl<W1 : Write, W2 : Write> Write for LogWriter<W1, W2>
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer1.write_all(buf)?;
        self.writer2.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer1.flush()?;
        self.writer2.flush()?;
        Ok(())
    }
}


// Patron Singleton
type Logger = LogWriter<std::io::Stdout, std::fs::File>;

pub static LOGGER : LazyLock<Mutex<Logger>> = LazyLock::new(|| {
     let log_file = std::fs::File::create("salida.log").expect("No se puedo crear el archivo de log");
     let stdout = std::io::stdout();

     Mutex::new(LogWriter {
         writer1: stdout,
         writer2: log_file,
     })
});