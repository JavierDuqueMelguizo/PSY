
use std::{io::Write};
use std::collections::HashMap;
use tokio_stream::StreamExt;

use zbus::{zvariant::{LE, OwnedObjectPath, OwnedFd, OwnedValue, Value, serialized::{Context, Data}}};

use super::{IScreenCastProxy, IRequestProxy};
use crate::utils::{self, logging::LOGGER};

// const APP_NAME : &str = "pipewire_yolo_streaming";
const RESTORE_FILE_PATH: &str = "restore_data.bin";

pub struct ScreenCastRunnerClient<'a> {
    connection: &'a zbus::Connection,
    proxy: IScreenCastProxy<'a>
}

impl<'a> ScreenCastRunnerClient<'a> {
    pub async fn new(connection: &'a zbus::Connection) -> zbus::Result<Self> {
        let proxy = IScreenCastProxy::new(connection).await?;
        Ok(Self { connection, proxy})
    }

    async fn get_request_proxy(
        &self,
        path: OwnedObjectPath,
    ) -> zbus::Result<IRequestProxy<'a>> {
        IRequestProxy::builder(self.connection)
            .path(path)?
            .build()
            .await
    }

    // async fn get_sender(
    //   &self,
    // ) -> String {
    //   let unique_name = self.connection.unique_name()
    //     .ok_or_else(|| zbus::Error::Failure("No unique D-Bus name found".into()))
    //     .expect("Error obteniendo unique D-Bus name");
    //   unique_name.as_str().trim_start_matches(':').replace('.', "_")
    // }

    // async fn build_request_path(
    //   &self,
    //   request_path: &str
    // ) -> Result<OwnedObjectPath, zbus::zvariant::Error> {
    //   let sender = self.get_sender().await;
    //   OwnedObjectPath::try_from(format!(
    //       "/org/freedesktop/portal/desktop/request/{sender}/{request_path}"
    //   ))
    // }

    pub async fn get_pipewire_node_id(&self) -> Result<(u32, OwnedFd), Box<dyn std::error::Error>>{
            
      // let base_path = format!("/org/freedesktop/portal/desktop/request/{}", APP_NAME );
      
      // Handles únicos para la petición
      let request_path_create_session = utils::generate_uuid_v4();
      let request_path_select_sources = utils::generate_uuid_v4();
      let request_path_start = utils::generate_uuid_v4();
      let session_path =utils::generate_uuid_v4();

      // 1. Crear la sesión de screencast
      let session_handler = self.create_session(request_path_create_session.as_str(), session_path.as_str()).await?;
      
      // 2. Seleccionar las fuentes (monitores/ventanas)
      let mut select_opts = HashMap::new();
      select_opts.insert("types", OwnedValue::from(2u32)); // 2 = Source Window
      select_opts.insert("cursor_mode", OwnedValue::from(1u32)); // 1 = Embedded cursor
      select_opts.insert("persist_mode", OwnedValue::from(2u32)); // 2 = Permissions persist until explicitly revoked

      self.select_sources(request_path_select_sources.as_str(), session_handler.clone(), select_opts).await?;
    
      // 3. Iniciar la sesión de screencast para generar y obtener el node_id
      let node_id = self.start(request_path_start.as_str(), session_handler.clone(), None).await?;
      
      //4. Abrir fd con Pipewire
      let pipewire_fd = self.open_pipewire_node(session_handler, HashMap::new()).await?;
      
      return Ok((node_id, pipewire_fd));
    }

    pub async fn create_session(
        &self,
        request_path: &str,
        session_path: &str,
    ) -> zbus::Result<OwnedObjectPath> {
      let options :  HashMap<&str, Value> = HashMap::from([
        ("handle_token", Value::from(request_path)),
        ("session_handle_token", Value::from(session_path) )
      ]);
      let mut logger = LOGGER.lock().await;
  
      let request_handler = self.proxy.create_session(options).await?;
      writeln!(logger, "CreateSession -> Response:{request_handler:?}")?;

      // Obtener el resultado quem e interesa:
      // (el 'session_handler' que me será necesario inmediatamente despues )
      let request_proxy = self.get_request_proxy(request_handler).await?;
      let mut signals = request_proxy.receive_response().await?;
      let session_handler : OwnedObjectPath  = loop {
        let signal = match signals.next().await {
          Some(response) => response,
          None => panic!("El stream termino sin generar señal")
        };
        let input = signal.args()?;
        writeln!(logger, "Request::Response -> Code:{}, Message:{}", input.response(), signal.message())?;

        let results = input.results();
        let handler = results.get("session_handle").expect("No se ha podido obtener 'session_handle'");
        break match handler {
          Value::ObjectPath(p) => p.clone().into(),
          Value::Str(s) => OwnedObjectPath::try_from(s.as_str())?,
          _ => {
              return Err(zbus::Error::Failure(
                  "El campo 'session_handle' no es Str ni ObjectPath".into(),
              ))
          }
        };
      };
      return Ok(session_handler);
    }

    pub async fn select_sources(
        &self,
        request_path: &str,
        session_handle: OwnedObjectPath,
        options : HashMap<&str, OwnedValue>
    ) -> zbus::Result<OwnedObjectPath> {
      let mut logger = LOGGER.lock().await;

      let mut _options : HashMap<&str, Value> = options.into_iter().map(
        |(key,value)| (key, Value::from(value))
      ).collect();
      _options.insert("handle_token", Value::from(request_path));
      // Si tenemos, el token "restore_data" guardado, lo reutilizamos
      if let Some(restore_data) = utils::load_data(RESTORE_FILE_PATH).await {
        let ctxt = Context::new_dbus(LE, 0);
        let (value , _): (OwnedValue, _) = Data::new(restore_data, ctxt).deserialize().expect("Error deserializando 'restore_data'");
        writeln!(logger, "Reutilizando restore_data guardado previamente...")?;
        writeln!(logger, "{}", serde_json::to_string(&value).expect("No se pudo serializar 'restore_data'"))?;
        _options.insert("restore_token", Value::from(value));
      }
      Ok(self.proxy.select_sources( session_handle, _options).await?)
    }

    pub async fn start(
        &self,
        request_path: &str,
        session_handle: OwnedObjectPath,
        parent_window: Option<&str>,
    ) -> zbus::Result<u32> {
      let mut logger = LOGGER.lock().await;

      let _parent_window = match parent_window{
        Some(value) => value,
        None => ""
      };
      let options :  HashMap<&str, Value> = HashMap::from([
        ("handle_token", Value::from(request_path)),
      ]);
      let request_handler = self.proxy.start(session_handle, _parent_window, options).await?;
      writeln!(logger, "Start -> Response:{request_handler:}")?;

      let request_proxy = self.get_request_proxy(request_handler).await?;
      let mut signals = request_proxy.receive_response().await?;

      let node_id = loop {
        let signal = signals.next().await.expect("El stream terminó sin recibir señal");
        let input = signal.args()?;
        writeln!(logger, "Request::Response -> Code:{}, Message:{}", input.response(), signal.message())?;

        let results = input.results();
        
        // Guardamos el restore token (si lo hubiera)
        if let Some(restore_token) = results.get("restore_token") {
          let ctxt = Context::new_dbus(LE, 0);
          let bytes = zbus::zvariant::to_bytes(ctxt, restore_token)?;
          if let Err(e) = utils::save_data(RESTORE_FILE_PATH, &bytes).await {
              writeln!(logger, "Error al guardar restore_token: {}", e)?;
          } else {
            // (vendor_name, version, implementation_data)
              writeln!(logger, "Nuevo restore_token guardado correctamente en disco.")?;
          }
        }

        // Extraemos el node_id generado
        if let Some(Value::Array(streams)) = results.get("streams") {
          if let Some(first_stream) = streams.get(0).ok() {
            if let Some(Value::Structure(s)) = first_stream {
              let fields = s.fields();
              if let Some(Value::U32(node_id)) = fields.get(0) {
                break node_id.clone();
              }
            }
          }
        }

      };

      return Ok(node_id);

    }

    pub async fn open_pipewire_node(
      &self,
      session_handle: OwnedObjectPath,
      options : HashMap<&str, OwnedValue>
    ) -> zbus::Result<zbus::zvariant::OwnedFd>{ 
      let mut _options : HashMap<&str, Value> = options.into_iter().map(
        |(key,value)| (key, Value::from(value))
      ).collect();

      self.proxy.open_pipewire_remote(session_handle, _options).await
    }
}