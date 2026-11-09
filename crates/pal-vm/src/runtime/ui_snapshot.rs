//! Portable v15: button attachment/gating and per-record history icon state.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct SavedUi {
    run_skip: i32,
    invalid_remaining_ms: u32,
    children: BTreeMap<(i32, i32), i32>,
    config: history_icons::IconConfig,
    records: Vec<history_icons::RecordIcons>,
    current_checkpoint: i32,
}

impl ScriptRuntime {
    pub(super) fn capture_ui(&self) -> SavedUi {
        SavedUi {
            run_skip: self.button_run_skip,
            invalid_remaining_ms: self.system_button_blocked_at.map_or(0, |start| {
                self.button_input_block_duration
                    .saturating_sub(self.pal_time_ms.wrapping_sub(start))
            }),
            children: self.button_children.clone(),
            config: self.history_state.icons.config.clone(),
            records: self.history_state.icons.records.clone(),
            current_checkpoint: self.history_state.icons.current_checkpoint,
        }
    }

    pub(super) fn restore_ui(&mut self, saved: Option<&SavedUi>) {
        if let Some(saved) = saved {
            self.button_run_skip = saved.run_skip;
            self.button_input_block_duration = saved.invalid_remaining_ms;
            self.system_button_blocked_at =
                (saved.invalid_remaining_ms != 0).then_some(self.pal_time_ms);
            self.button_children = saved.children.clone();
            let current = &self.history_state.icons.config;
            self.history_state.icons.config = history_icons::IconConfig {
                separator_enabled: current.separator_enabled,
                separator_name: current.separator_name.clone(),
                separator_position: current.separator_position,
                ..saved.config.clone()
            };
            self.history_state.icons.records = saved.records.clone();
            self.history_state.icons.current_checkpoint = saved.current_checkpoint;
            self.restore_history_counter(saved.current_checkpoint);
        } else {
            self.button_run_skip = 0;
            self.button_children.clear();
            self.history_state.icons.records.clear();
        }
        self.history_state
            .icons
            .records
            .resize(self.history_state.records.len(), Default::default());
        self.history_state.icons.invalidate();
    }
}

pub(super) fn write(out: &mut Vec<u8>, saved: Option<&SavedUi>) -> std::io::Result<()> {
    write_u32(out, u32::from(saved.is_some()))?;
    let Some(saved) = saved else {
        return Ok(());
    };
    write_i32(out, saved.run_skip)?;
    write_u32(out, saved.invalid_remaining_ms)?;
    write_u32(out, saved.children.len() as u32)?;
    for (&(group, child), &parent) in &saved.children {
        for word in [group, child, parent] {
            write_i32(out, word)?;
        }
    }
    let c = &saved.config;
    for word in [
        i32::from(c.face_enabled),
        i32::from(c.face_capture),
        c.face_position[0],
        c.face_position[1],
        c.face_group,
        c.face_click,
        i32::from(c.secondary_enabled),
        c.secondary_position[0],
        c.secondary_position[1],
        c.secondary_group,
        c.secondary_click,
        c.secondary_hover,
        c.secondary_update,
        i32::from(c.voice_icon_enabled),
    ] {
        write_i32(out, word)?;
    }
    write_bytes(out, c.secondary_name.as_bytes(), SAVE_NAME_CAP)?;
    write_bytes(out, c.voice_icon_name.as_bytes(), SAVE_NAME_CAP)?;
    write_i32(out, saved.current_checkpoint)?;
    write_u32(out, saved.records.len() as u32)?;
    for record in &saved.records {
        write_u32(out, u32::from(record.face))?;
        write_i32(out, record.checkpoint)?;
    }
    Ok(())
}

pub(super) fn read(reader: &mut impl Read) -> std::io::Result<Option<SavedUi>> {
    if read_u32_from(reader)? == 0 {
        return Ok(None);
    }
    let mut saved = SavedUi {
        run_skip: read_i32_from(reader)?,
        invalid_remaining_ms: read_u32_from(reader)?,
        ..Default::default()
    };
    let count = read_u32_from(reader)?;
    if count > 2048 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "too many button attachments",
        ));
    }
    for _ in 0..count {
        let group = read_i32_from(reader)?;
        let child = read_i32_from(reader)?;
        let parent = read_i32_from(reader)?;
        if group as u32 >= 16 || child as u32 >= 128 || parent as u32 >= 128 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid saved button attachment",
            ));
        }
        saved.children.insert((group, child), parent);
    }
    let mut words = [0; 14];
    for word in &mut words {
        *word = read_i32_from(reader)?;
    }
    let mut name = || -> std::io::Result<String> {
        String::from_utf8(read_bytes_capped(reader, SAVE_NAME_CAP)?)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    };
    saved.config = history_icons::IconConfig {
        face_enabled: words[0] != 0,
        face_capture: words[1] != 0,
        face_position: [words[2], words[3]],
        face_group: words[4],
        face_click: words[5],
        secondary_enabled: words[6] != 0,
        secondary_position: [words[7], words[8]],
        secondary_group: words[9],
        secondary_click: words[10],
        secondary_hover: words[11],
        secondary_update: words[12],
        voice_icon_enabled: words[13] != 0,
        secondary_name: name()?,
        voice_icon_name: name()?,
        // Not part of the save image: the separator line is boot config.
        ..Default::default()
    };
    saved.current_checkpoint = read_i32_from(reader)?;
    let count = read_u32_from(reader)?;
    if count > 128 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "too many history icon records",
        ));
    }
    for _ in 0..count {
        saved.records.push(history_icons::RecordIcons {
            face: read_u32_from(reader)? != 0,
            checkpoint: read_i32_from(reader)?,
        });
    }
    Ok(Some(saved))
}

pub(super) fn write_strings(
    out: &mut Vec<u8>,
    strings: Option<&(Vec<String>, usize)>,
) -> std::io::Result<()> {
    write_u32(out, u32::from(strings.is_some()))?;
    let Some((strings, cursor)) = strings else {
        return Ok(());
    };
    if strings.len() > 65_536
        || strings.iter().any(|s| s.len() > 1024 * 1024)
        || strings.iter().map(String::len).sum::<usize>() > 64 * 1024 * 1024
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "dynamic string bank exceeds save limit",
        ));
    }
    write_u32(
        out,
        u32::try_from(*cursor)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
    )?;
    write_u32(out, strings.len() as u32)?;
    for string in strings {
        write_bytes(out, string.as_bytes(), 1024 * 1024)?;
    }
    Ok(())
}

pub(super) fn read_strings(
    reader: &mut impl Read,
) -> std::io::Result<Option<(Vec<String>, usize)>> {
    if read_u32_from(reader)? == 0 {
        return Ok(None);
    }
    let cursor = read_u32_from(reader)? as usize;
    let count = read_u32_from(reader)?;
    if count > 65_536 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "too many dynamic strings",
        ));
    }
    let mut strings = Vec::with_capacity(count as usize);
    let mut total = 0usize;
    for _ in 0..count {
        let bytes = read_bytes_capped(reader, 1024 * 1024)?;
        total += bytes.len();
        if total > 64 * 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "dynamic string bank exceeds save limit",
            ));
        }
        strings.push(
            String::from_utf8(bytes)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?,
        );
    }
    Ok(Some((strings, cursor)))
}
