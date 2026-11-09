//! Icone della shell (le stesse di Esplora file) convertite in bitmap
//! 32 bit con alpha, il formato che i menu Win32 sanno disegnare.

use std::collections::HashMap;
use std::ffi::c_void;
use std::path::Path;
use std::ptr::null_mut;

use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GdiFlush, HBITMAP, SelectObject,
};
use windows_sys::Win32::UI::Shell::{
    SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SMALLICON, SHGetFileInfoW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DI_NORMAL, DestroyIcon, DrawIconEx, GetSystemMetrics, HICON, SM_CXSMICON,
};

use crate::sys::wide;

/// Estensioni la cui icona dipende dal singolo file e non dal tipo.
const PER_FILE_EXTENSIONS: &[&str] = &["exe", "lnk", "url", "ico", "cur", "ani", "appref-ms", "msc", "scr"];

pub struct IconCache {
    bitmaps: HashMap<String, HBITMAP>,
    size: i32,
}

impl IconCache {
    pub fn new() -> Self {
        Self { bitmaps: HashMap::new(), size: unsafe { GetSystemMetrics(SM_CXSMICON) } }
    }

    /// Bitmap piccola per la voce di menu; null se non disponibile.
    /// Le bitmap restano in cache per tutta la vita del processo.
    pub fn menu_bitmap(&mut self, path: &Path, is_dir: bool) -> HBITMAP {
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let key = if is_dir || PER_FILE_EXTENSIONS.contains(&ext.as_str()) {
            path.to_string_lossy().to_lowercase()
        } else {
            format!("ext:{ext}")
        };

        if let Some(&bitmap) = self.bitmaps.get(&key) {
            return bitmap;
        }

        let icon = shell_icon(path, true);
        let bitmap = if icon.is_null() {
            null_mut()
        } else {
            let bitmap = icon_to_bitmap(icon, self.size);
            unsafe { DestroyIcon(icon) };
            bitmap
        };
        self.bitmaps.insert(key, bitmap);
        bitmap
    }
}

impl Drop for IconCache {
    fn drop(&mut self) {
        for &bitmap in self.bitmaps.values().filter(|b| !b.is_null()) {
            unsafe { DeleteObject(bitmap) };
        }
    }
}

/// Icona della shell per un percorso; il chiamante deve distruggerla con `DestroyIcon`.
pub fn shell_icon(path: &Path, small: bool) -> HICON {
    let path = wide(path);
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let flags = SHGFI_ICON | if small { SHGFI_SMALLICON } else { SHGFI_LARGEICON };
    let result = unsafe {
        SHGetFileInfoW(path.as_ptr(), 0, &mut info, size_of::<SHFILEINFOW>() as u32, flags)
    };
    if result == 0 { null_mut() } else { info.hIcon }
}

fn icon_to_bitmap(icon: HICON, size: i32) -> HBITMAP {
    unsafe {
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = size;
        info.bmiHeader.biHeight = -size; // top-down
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;

        let dc = CreateCompatibleDC(null_mut());
        let mut bits: *mut c_void = null_mut();
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bitmap.is_null() || bits.is_null() {
            DeleteDC(dc);
            return null_mut();
        }

        let previous = SelectObject(dc, bitmap);
        DrawIconEx(dc, 0, 0, icon, size, size, 0, null_mut(), DI_NORMAL);
        SelectObject(dc, previous);
        DeleteDC(dc);
        GdiFlush();

        // Icone vecchie senza canale alpha: si rendono opachi i pixel disegnati.
        let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (size * size) as usize);
        if pixels.iter().all(|p| p >> 24 == 0) {
            for p in pixels.iter_mut().filter(|p| **p & 0x00FF_FFFF != 0) {
                *p |= 0xFF00_0000;
            }
        }

        bitmap
    }
}
