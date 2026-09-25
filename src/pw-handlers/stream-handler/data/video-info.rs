use crate::pw_handlers::KeyValue;



// ToDo: Remplazar por VideoInfoRaw de Pipewire.
// Actualmente no esta implementado y no se utiliza...
// La idea es tener una opción para almacenar flags tambien...
struct VideoInfo{
    format : KeyValue,
    flags: KeyValue,
    modifier: KeyValue,
    size: KeyValue,
    framerate: KeyValue,
    max_framerate: KeyValue,
    views: KeyValue,
    interlace_mode: KeyValue,
}