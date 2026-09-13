
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


/// MACROS
#[macro_export]
macro_rules! log_write_async{
    () => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        let mut logger = LOGGER.lock().await;
        write!(logger, "\n")
    }};
    ($($arg:tt)*) => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        let mut logger = LOGGER.lock().await;
        write!(logger, $($arg)*)
    }};
}
#[macro_export]
macro_rules! log_write{
    
    () => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        if let Some(logger) = LOGGER.try_lock().ok().as_mut(){
            let _ = write!(logger, "\n")
        }
        else{
            print!("\n");
        }
        
    }};
    ($($arg:tt)*) => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        if let Some(logger) = LOGGER.try_lock().ok().as_mut(){
            let _ = write!(logger, $($arg)*)
        }
        else{
            println!($($arg)*);
        }
        
    }};
}


#[macro_export]
macro_rules! log_writeln_async{
    () => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        let mut logger = LOGGER.lock().await;
        writeln!(logger, "\n")
    }};
    ($($arg:tt)*) => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        let mut logger = LOGGER.lock().await;
        writeln!(logger, $($arg)*)
    }};
}
#[macro_export]
macro_rules! log_writeln{
    () => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};

        if let Some(logger) = LOGGER.try_lock().ok().as_mut(){
            let _ = writeln!(logger, "\n");
        }
        else{
            print!("\n");
            //Err(std::io::Error::new(std::io::ErrorKind::ResourceBusy, "El recurso no se encuentra"))
        }
        
    }};
    
    ($($arg:tt)*) => {{
        use std::{io::Write};
        use crate::utils::logging::{LOGGER};


        if let Some(logger) = LOGGER.try_lock().ok().as_mut(){
            let _ = writeln!(logger, $($arg)*);
        }
        else{
            println!($($arg)*);
            //Err(std::io::Error::new(std::io::ErrorKind::ResourceBusy, "El recurso no se encuentra"))
        }
        
    }};
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


