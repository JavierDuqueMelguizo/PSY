
use zbus::{Connection, zvariant::OwnedFd};

mod utils;

mod dbus;
use dbus::{krunner::WindowsRunnerClient, screencast::ScreenCastRunnerClient};

use crate::pw_handlers::{PipewireHandlerBuilder, StreamProcessorPipewireBuilder, VideoProcessorGPU};

mod pw_handlers;

// No usandose:
// mod wayland;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let connection = Connection::session().await?;
    
    // 1. Listar ventanas disponibles (opcional)...
    let _ :  Result<(), Box<dyn std::error::Error>> = 
    {
        // let mut logger = LOGGER.lock().await;
        let client = WindowsRunnerClient::new(&connection).await?;

        let windows = client.get_active_windows().await?;
        log_write_async!("--- Ventanas Abiertas ({}) ---", windows.len())?;
        for win in windows {
            log_write_async!("• [{}] {} {} ({} {})", win.id, win.title, win.app_id, win.category, win.relevance)?;
            log_write_async!("  Propiedades[Keys]: {:#?}", win.properties.into_keys().collect::<Vec<_>>())?;
        }
        // logger.flush()?;

        Ok(())
    };

    // 2. Levantar ScreenPortal para seleccionar fuente del streaming...
    let result :  Result<(u32, OwnedFd), Box<dyn std::error::Error>> = {
        let client = ScreenCastRunnerClient::new(&connection).await?;
        //let window_uuid = "f630d34f-463a-4743-923c-cb829514f7a8";
        // let mut input: String = String::new();
        // println!("Ingrese el UUID de la ventana a capturar (ej: f630d34f-463a-4743-923c-cb829514f7a8): ");
        // std::io::stdin() 
        //     .read_line(&mut input) 
        //     .expect("Unable to read Stdin");
        
        let result = client.get_pipewire_node_id().await?;

        log_write_async!("--- PipeWire Node ID para la ventana: {} , fd: {:?} ---", result.0, result.1)?;

        Ok(result)
    };

    // 3. Llamar a Pipewire para que empiece a grabar de la fuente y poder leer datos...
    let _ : Result<(), Box<dyn std::error::Error>> = {
        let (node, fd) = result?;
        let std_fd : std::os::fd::OwnedFd = fd.into();
        let mut stream_handler = PipewireHandlerBuilder::new(None)?
            .context(None)?
            .core(std_fd, None)?
            .build()?;
        let _ = stream_handler.default_video_stream(
            "pipewire-yolo-client", 
            Some(node),
            StreamProcessorPipewireBuilder::<VideoProcessorGPU>::new()
        )?;


        let controls = stream_handler.start();

        controls.1.await?;


        Ok(())
    };
 
    log_write_async!("--- TERMINANDO MAIN ---")?;

    connection.close().await?;
    unsafe{
        pipewire::deinit();
    }

    Ok(())
}


/*
# Primero verifico que estoy en Wayland.

```
$> echo $XDG_SESSION_TYPE
wayland
```

# Quiero obtener la lista de  monitores y ventanas abiertas disponibles.
## Obtener la lista de monitores disponibles:
```
for output in /sys/class/drm/card*-*\/status; do echo "$output: $(cat $output)"; done
```

# Obtener la lista de ventanas abiertas:
```
busctl --user call org.kde.KWin /WindowsRunner org.kde.krunner1 Match s "" | gawk -v RS='"' 'NR%2==0' | printf "$(cat)" | grep -vE "^(subtext|icon-data|0_)"
```

*/