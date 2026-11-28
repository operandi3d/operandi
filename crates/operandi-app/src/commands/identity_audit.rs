//! Opt-in, read-only I1 evidence. This never dispatches commands or alters authored
//! data. External capture binds current_exe/PID to independently hashed bytes.
use super::*;
pub(super) fn capture(app: &mut Operandi, ctx: &Context) {
    let Some(path) = std::env::var_os("OPERANDI_I1_AUDIT").map(PathBuf::from) else {
        return;
    };
    if let Err(e) = app.editor.synchronize_identities() {
        app.status = format!("Identity audit: {e}");
        return;
    }
    let checkpoints = |cs: &[std::sync::Arc<Checkpoint>]| {
        cs.iter().map(|c|serde_json::json!({"label":c.label,"before":c.before,"after":c.after,"identity_before":c.identity_before,"identity_after":c.identity_after})).collect::<Vec<_>>()
    };
    let audit = serde_json::json!({
        "pid":std::process::id(),"executable":std::env::current_exe().ok(),
        "launcher_binary_sha256":std::env::var("OPERANDI_I1_BINARY_SHA256").ok(),
        "epoch":app.editor.identities.epoch,"publication":app.editor.identities.publication,"request":app.editor.identities.request,"next_id":app.editor.identities.next_id().ok(),
        "identities":app.editor.identities.current,"scene":app.editor.scene,
        "undo":checkpoints(&app.editor.history.undo),"redo":checkpoints(&app.editor.history.redo),
        "live":app.editor.session.live.as_ref().map(|l|serde_json::json!({"tool":l.tool.label(),"applied":l.applied,"extent":l.params.extent,"mode":format!("{:?}",l.params.extrude_mode),"sides":l.params.extrude_sides,"uvs":format!("{:?}",l.params.extrude_uvs),"preview_error":l.preview_error})),
        "i2":app.editor.i2.selected.as_ref().map(|s|serde_json::json!({"key":s.key,"anchor":s.anchor,"cursor":s.cursor,"ever_parked":s.ever_parked,"base":s.base.scene,"begun":s.begun.scene,"movement":s.movement.as_ref().map(|m|&m.scene),"base_ids":s.base.identities,"begun_ids":s.begun.identities,"frame":s.frame.map(|f|serde_json::json!({"center":f.center,"basis":f.basis})),"plan":s.plan,"armed_extent":s.armed_params.extent,"movement_extent":s.movement_params.as_ref().map(|p|p.extent)})),
        "i2_prior":app.editor.i2.prior.as_ref().map(|s|serde_json::json!({"key":s.key,"cursor":s.cursor})),
        "stage_next":app.editor.i2.serial.next().ok(),
        "viewport_rect":[app.viewport_rect.left(),app.viewport_rect.top(),app.viewport_rect.width(),app.viewport_rect.height()],
        "camera": {"distance":app.camera.distance,"view":format!("{:?}",app.camera.view),"target":app.camera.target,"yaw":app.camera.yaw,"pitch":app.camera.pitch,"basis": format!("{:?}", app.camera.basis())},
        "handles":app.handles(app.viewport_rect).iter().map(|(axis,a,b,label)|serde_json::json!({"axis":axis,"start":[a.x,a.y],"end":[b.x,b.y],"label":label})).collect::<Vec<_>>(),
        "field_rects":app.field_rects.iter().map(|(name,r)|(name.clone(),[r.left(),r.top(),r.width(),r.height()])).collect::<BTreeMap<_,_>>(),
        "input_map":app.bindings,"capture_active":app.navigation.is_some(),"field_focus":app.field_focus.map(|id|format!("{id:?}")),
        "gesture_ticket":app.i2_gesture_ticket,
        "path":app.path,"dirty":app.dirty(),"status":app.status,"fields":app.fields,"confirmed_fields":app.confirmed_fields,"field_errors":app.field_errors,
        "pixels_per_point":ctx.pixels_per_point(),"gpu":app.gpu
    });
    let Ok(bytes) = serde_json::to_vec_pretty(&audit) else {
        return;
    };
    if std::fs::read(&path).ok().as_deref() == Some(bytes.as_slice()) {
        return;
    }
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    if std::fs::write(&temp, bytes).is_ok() {
        let _ = std::fs::rename(temp, path);
    }
}
