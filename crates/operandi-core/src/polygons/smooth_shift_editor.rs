use crate::*;
impl Editor {
    /// Session-only settings and named states are separate from scene/history.
    pub(crate) fn dispatch_surface(&mut self, cmd: Command) -> Result<()> {
        match cmd {
            Command::ConfigureSmoothShift(p) => self.configure_surface(p)?,
            Command::ResetSmoothShift => {
                let l = self
                    .session
                    .live
                    .as_ref()
                    .ok_or("Arm Smooth Shift or Thicken")?;
                let key = l.tool.label();
                // TP-15: with no named state, Reset gives the tool's own
                // starting values; Thicken's come from its preset row.
                let p = self
                    .surface_state
                    .get(key)
                    .and_then(|n| self.surface_states.get(key)?.get(n))
                    .copied()
                    .unwrap_or_else(|| {
                        if l.tool == Tool::Thicken {
                            crate::tool_preset::thicken_values()
                        } else {
                            SmoothShiftParameters::default()
                        }
                    });
                self.configure_surface(p)?;
            }
            Command::CloneSmoothShift => {
                let l = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_smooth_shift())
                    .ok_or("Arm Smooth Shift or Thicken")?;
                let p = l.params.smooth_shift;
                let mut staged = self.clone();
                staged.dispatch(Command::Continue)?;
                staged.configure_surface(p)?;
                let p = staged.session.live.as_ref().unwrap().params.clone();
                staged.dispatch(Command::Preview(p))?;
                *self = staged;
            }
            Command::SurfaceStateSave(name) => {
                if name.trim().is_empty() || name.len() > 256 {
                    return Err("Use a Tool State name of 1–256 bytes".into());
                }
                let l = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_smooth_shift())
                    .ok_or("Arm a surface tool")?;
                self.surface_states
                    .entry(l.tool.label().into())
                    .or_default()
                    .insert(name.clone(), l.params.smooth_shift);
                self.surface_state.insert(l.tool.label().into(), name);
            }
            Command::SurfaceStateRecall(name) => {
                let key = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_smooth_shift())
                    .ok_or("Arm a surface tool")?
                    .tool
                    .label()
                    .to_string();
                if let Some(name) = name {
                    let p = *self
                        .surface_states
                        .get(&key)
                        .and_then(|m| m.get(&name))
                        .ok_or("Missing Tool State")?;
                    self.configure_surface(p)?;
                    self.surface_state.insert(key, name);
                } else {
                    self.surface_state.remove(&key);
                }
            }
            Command::SurfaceStateDelete => {
                let key = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_smooth_shift())
                    .ok_or("Arm a surface tool")?
                    .tool
                    .label()
                    .to_string();
                if let Some(n) = self.surface_state.remove(&key)
                    && let Some(m) = self.surface_states.get_mut(&key)
                {
                    m.remove(&n);
                }
            }
            Command::SurfaceStateRename(name) => {
                if name.trim().is_empty() || name.len() > 256 {
                    return Err("Use a Tool State name of 1–256 bytes".into());
                }
                let key = self
                    .session
                    .live
                    .as_ref()
                    .filter(|l| l.tool.is_smooth_shift())
                    .ok_or("Arm a surface tool")?
                    .tool
                    .label()
                    .to_string();
                let old = self
                    .surface_state
                    .get(&key)
                    .ok_or("Select a Tool State")?
                    .clone();
                let m = self
                    .surface_states
                    .get_mut(&key)
                    .ok_or("Missing Tool State")?;
                if m.contains_key(&name) {
                    return Err("Tool State already exists".into());
                }
                let p = m.remove(&old).ok_or("Missing Tool State")?;
                m.insert(name.clone(), p);
                self.surface_state.insert(key, name);
            }
            _ => return Err("Expected a Smooth Shift/Thicken session command".into()),
        }
        Ok(())
    }
    pub(crate) fn retain_surface(&mut self, tool: Tool, p: SmoothShiftParameters) {
        if tool == Tool::Thicken {
            *self.presets.get_mut::<crate::ThickenPreset>() = Some(p);
        } else {
            *self.presets.get_mut::<crate::SmoothShiftPreset>() = p;
        }
    }
    pub(crate) fn configure_surface(&mut self, p: SmoothShiftParameters) -> Result<()> {
        if let Err(e) = p.validate() {
            if let Some(l) = self
                .session
                .live
                .as_mut()
                .filter(|l| l.tool.is_smooth_shift())
            {
                l.preview_error = Some(e.clone());
            }
            return Err(e);
        }

        let live = self
            .session
            .live
            .as_ref()
            .filter(|l| l.tool.is_smooth_shift())
            .ok_or("Arm Smooth Shift or Thicken")?;
        let tool = live.tool;
        let mut params = live.params.clone();
        params.smooth_shift = p;
        if live.applied {
            self.dispatch(Command::Preview(params))?;
        } else {
            let l = self.session.live.as_mut().unwrap();
            l.params = params;
            l.preview_error = None;
        }
        self.retain_surface(tool, p);
        Ok(())
    }
}
