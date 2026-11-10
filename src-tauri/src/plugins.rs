//! Plugin catalog: scans `plugins/` next to the app, inspects `.rpy`, folders and
//! zip packages, and copies a plugin's game files into the project.

use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginFile {
    pub source: String,
    pub target: String,
    pub size: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub source: String,
    pub size: u64,
    pub files: Vec<PluginFile>,
    pub snippet: Option<String>,
    pub docs: Option<String>,
    pub enabled: bool,
}

struct CatalogEntry {
    matcher: &'static str,
    name: &'static str,
    description: &'static str,
    snippet: Option<&'static str>,
}

const CATALOG: &[CatalogEntry] = &[
    CatalogEntry { matcher: "strat", name: "STRAT вЂ” РїРµСЂРµС…РѕРґ-С‚СЂСЏСЃРєР°", description: "РЎР»СѓС‡Р°Р№РЅР°СЏ С‚СЂСЏСЃРєР° РїСЂРё РїРµСЂРµС…РѕРґРµ, РєР°Рє С‚РµР»РµРїРѕСЂС‚. РџР°СЂР°РјРµС‚СЂС‹: strat(t, x, y).", snippet: Some("with strat(0.6, 12, 8)") },
    CatalogEntry { matcher: "flip3d", name: "FLIP3D вЂ” 3D-РїРµСЂРµС…РѕРґС‹", description: "Р§РµС‚С‹СЂРµ РїРµСЂРµС…РѕРґР°-РІСЂР°С‰РµРЅРёСЏ РєСѓР±Р°, РјРѕР¶РЅРѕ РїРѕРґРєР»СЋС‡РёС‚СЊ Р·РІСѓРє РїРѕРІРѕСЂРѕС‚РѕРІ.", snippet: Some("with flip3d") },
    CatalogEntry { matcher: "bub", name: "Bub вЂ” Р±Р°Р±Р»С‹ Сѓ СЃРїСЂР°Р№С‚РѕРІ", description: "Р РµРїР»РёРєРё РІ РїСѓР·С‹СЂСЏС…, РїСЂРёРІСЏР·Р°РЅРЅС‹С… Рє СЃРїСЂР°Р№С‚Сѓ. РќР°СЃС‚СЂР°РёРІР°СЋС‚СЃСЏ С†РІРµС‚, РѕС‚СЃС‚СѓРїС‹ Рё С„РѕРЅ.", snippet: Some("$ big = Bub(_(\"Р—РґРѕСЂРѕРІСЏРє\"))\nshow big ok\nbig \"РЈСЏСЃРµ, РєР°РєРѕР№ Р·РґРѕСЂРѕРІРµРЅРЅС‹Р№!\"") },
    CatalogEntry { matcher: "beep", name: "BEEP вЂ” РїСЃРµРІРґРѕ-РѕР·РІСѓС‡РєР°", description: "РџРёСЃРє/Р±РѕСЂРјРѕС‚Р°РЅРёРµ РїСЂРё РІС‹РІРѕРґРµ С‚РµРєСЃС‚Р° РґР»СЏ РІСЃРµС… РїРµСЂСЃРѕРЅР°Р¶РµР№, РєСЂРѕРјРµ СЂР°СЃСЃРєР°Р·С‡РёРєР°.", snippet: None },
    CatalogEntry { matcher: "auto_bubbles", name: "Bub вЂ” Р±Р°Р±Р»С‹ (РїСЂРёРјРµСЂ)", description: "Р”РµРјРѕ-РїСЂРѕРµРєС‚ Р±Р°Р±Р»РѕРІ Сѓ СЃРїСЂР°Р№С‚РѕРІ (bub.rpy + РїСЂРёРјРµСЂС‹).", snippet: Some("$ big = Bub(_(\"Р—РґРѕСЂРѕРІСЏРє\"))\nshow big ok\nbig \"РЈСЏСЃРµ, РєР°РєРѕР№ Р·РґРѕСЂРѕРІРµРЅРЅС‹Р№!\"") },
    CatalogEntry { matcher: "bw_shader", name: "BW Shader вЂ” С‡/Р± РјР°РЅРіР°", description: "Р§С‘СЂРЅРѕ-Р±РµР»С‹Р№ С€РµР№РґРµСЂ СЃ РґРёР·РµСЂРёРЅРіРѕРј РґР»СЏ СЃРїСЂР°Р№С‚Р° РёР»Рё СЃР»РѕСЏ master.", snippet: None },
    CatalogEntry { matcher: "flashlight", name: "FLASHLIGHT вЂ” С„РѕРЅР°СЂРёРє", description: "РЁРµР№РґРµСЂ С„РѕРЅР°СЂРёРєР° СЃ Р·Р°СЃРІРµС‚РєРѕР№; РґРѕРїРѕР»РЅРёС‚РµР»СЊРЅС‹С… РєР°СЂС‚РёРЅРѕРє РЅРµ РЅСѓР¶РЅРѕ.", snippet: None },
    CatalogEntry { matcher: "myframe", name: "MYFRAME + PIXELIZE", description: "Р Р°СЃС€РёСЂРµРЅРёРµ Frame СЃ РѕР±РІРѕРґРєРѕР№ Рё РїРёРєСЃРµР»СЊРЅС‹Р№ СЂРµР¶РёРј.", snippet: None },
    CatalogEntry { matcher: "pixelize", name: "PIXELIZE вЂ” РїРёРєСЃРµР»СЊРЅС‹Р№ СЂРµР¶РёРј", description: "РџРёРєСЃРµР»РёР·Р°С†РёСЏ РёР·РѕР±СЂР°Р¶РµРЅРёСЏ Рё С‚РµРєСЃС‚Р°.", snippet: None },
    CatalogEntry { matcher: "autoparallax", name: "AUTOPARALLAX вЂ” РїР°СЂР°Р»Р»Р°РєСЃ", description: "РџР°СЂР°Р»Р»Р°РєСЃ РІ Р»СЋР±РѕР№ РЅРѕРІРµР»Р»Рµ: С„РѕРЅС‹ РґР°Р»СЊС€Рµ, СЃРїСЂР°Р№С‚С‹ Р±Р»РёР¶Рµ. Р‘РµР· РєРѕРґР°.", snippet: None },
    CatalogEntry { matcher: "panorama", name: "Panorama вЂ” РїР°РЅРѕСЂР°РјР° Р·Р° РєСѓСЂСЃРѕСЂРѕРј", description: "РЁРёСЂРѕРєРёР№ С„РѕРЅ РґРІРёРіР°РµС‚СЃСЏ РІСЃР»РµРґ Р·Р° РєСѓСЂСЃРѕСЂРѕРј.", snippet: None },
    CatalogEntry { matcher: "steps", name: "Steps вЂ” С€Р°РіРё Рё РґРІРёР¶СѓС‰РёР№СЃСЏ С„РѕРЅ", description: "РЎРїСЂР°Р№С‚С‹ С€Р°РіР°СЋС‚ РїРѕ СЌРєСЂР°РЅСѓ, РјРЅРѕРіРѕСЃР»РѕР№РЅС‹Р№ С„РѕРЅ РґРІРёР¶РµС‚СЃСЏ РјРёРјРѕ.", snippet: None },
    CatalogEntry { matcher: "stereo", name: "Stereo вЂ” Р·РІСѓРє Р·Р° СЃРїСЂР°Р№С‚Р°РјРё", description: "Р‘Р°Р»Р°РЅСЃ РєР°РЅР°Р»РѕРІ СЃР»РµРґСѓРµС‚ Р·Р° РїРѕР»РѕР¶РµРЅРёРµРј СЃРїСЂР°Р№С‚РѕРІ (СЂР°Р±РѕС‚Р°РµС‚ СЃ 7DOTS).", snippet: None },
    CatalogEntry { matcher: "oldtv", name: "OldTV вЂ” СЃС‚Р°СЂС‹Р№ С‚РµР»РµРІРёР·РѕСЂ", description: "Р РµР¶РёРј СЃС‚Р°СЂРѕРіРѕ С‚РµР»РµРІРёР·РѕСЂР° РґР»СЏ С„Р»СЌС€Р±СЌРєРѕРІ, РјРЅРѕРіРѕ РЅР°СЃС‚СЂРѕРµРє.", snippet: None },
    CatalogEntry { matcher: "shaders2", name: "Simple Shaders 2 вЂ” РєРѕСЃРјРѕСЃ", description: "РЁРµР№РґРµСЂС‹ РїРѕР»С‘С‚Р° СЃСЂРµРґРё Р·РІС‘Р·Рґ, РіР°Р»Р°РєС‚РёРєРё Рё РІРѕР»РЅ.", snippet: None },
    CatalogEntry { matcher: "texture_pixel", name: "Texture Pixel", description: "Р—Р°РјРµРЅР° РїРёРєСЃРµР»РµР№ РєР°СЂС‚РёРЅРєР°РјРё РёР»Рё СЃРёРјРІРѕР»Р°РјРё, СЂРµРіСѓР»РёСЂРѕРІРєР° СЂР°Р·РјРµСЂР° Рё СЏСЂРєРѕСЃС‚Рё.", snippet: None },
    CatalogEntry { matcher: "simple_inventory", name: "Simple Inventory", description: "РџСЂРѕСЃС‚РµР№С€РёР№ РёРЅРІРµРЅС‚Р°СЂСЊ: СЃР±РѕСЂ РїСЂРµРґРјРµС‚РѕРІ РІРѕ РІСЂРµРјСЏ С‡С‚РµРЅРёСЏ РЅРѕРІРµР»Р»С‹.", snippet: None },
    CatalogEntry { matcher: "quiz_timer", name: "Quiz Timer вЂ” РІРёРєС‚РѕСЂРёРЅР° СЃ С‚Р°Р№РјРµСЂРѕРј", description: "Р’РёРєС‚РѕСЂРёРЅР° СЃ С‚Р°Р№РјРµСЂРѕРј, Р·Р°РїСѓСЃРєР°РµС‚СЃСЏ РѕРґРЅРѕР№ РєРѕРјР°РЅРґРѕР№, СЃС‚РёР»Рё РЅР°СЃС‚СЂР°РёРІР°СЋС‚СЃСЏ.", snippet: None },
    CatalogEntry { matcher: "quiz", name: "Quiz вЂ” РІРёРєС‚РѕСЂРёРЅР° СЃРѕ СЃРІР°Р№РїР°РјРё", description: "РњРёРЅРё-РІРёРєС‚РѕСЂРёРЅР° Р”Рђ/РќР•Рў РєРЅРѕРїРєР°РјРё РёР»Рё СЃРІР°Р№РїР°РјРё.", snippet: None },
    CatalogEntry { matcher: "tasks_tabs", name: "Tasks Tabs вЂ” СЃРїРёСЃРѕРє Р·Р°РґР°С‡", description: "РЈР»СѓС‡С€РµРЅРЅС‹Р№ СЃРїРёСЃРѕРє Р·Р°РґР°С‡ СЃ РІРєР»Р°РґРєР°РјРё Рё РёРєРѕРЅРєР°РјРё.", snippet: None },
    CatalogEntry { matcher: "tasks", name: "Tasks вЂ” СЃРїРёСЃРѕРє Р·Р°РґР°С‡", description: "Р—Р°РґР°С‡Рё РїРѕ РіСЂСѓРїРїР°Рј, РёРєРѕРЅРєРё, РїРѕРґСЃРєР°Р·РєРё; СЂР°Р±РѕС‚Р°РµС‚ СЃ 7DOTS.", snippet: None },
    CatalogEntry { matcher: "info", name: "Info вЂ” СЃС‚Р°С‚С‹ Рё РјРµРЅСЋ", description: "РћРєРЅРѕ СЃРѕ СЃС‚Р°С‚Р°РјРё Рё РјРµРЅСЋ РІС‹Р±РѕСЂРѕРІ, РІС‹Р·С‹РІР°РµРјРѕРµ РёР· СЌРєСЂР°РЅРѕРІ.", snippet: None },
    CatalogEntry { matcher: "cho", name: "Cho вЂ” РєР°СЃС‚РѕРјРЅРѕРµ РјРµРЅСЋ", description: "РњРµРЅСЋ СЃ РїРѕР»РѕР¶РµРЅРёРµРј, С‚СЂР°РЅСЃС„РѕСЂРјР°РјРё Рё РЅРµР°РєС‚РёРІРЅС‹РјРё РєРЅРѕРїРєР°РјРё.", snippet: None },
    CatalogEntry { matcher: "myinput", name: "MyInput вЂ” РїРѕР»Рµ РІРІРѕРґР°", description: "РљР°СЃС‚РѕРјРЅРѕРµ РїРѕР»Рµ РІРІРѕРґР° myinput()/MyInput(), РґР°Р¶Рµ РёР· СЌРєСЂР°РЅРѕРІ.", snippet: Some("$ myinput()") },
    CatalogEntry { matcher: "inputs", name: "Inputs вЂ” РЅРµСЃРєРѕР»СЊРєРѕ РїРѕР»РµР№", description: "РќРµСЃРєРѕР»СЊРєРѕ РїРѕР»РµР№ РІРІРѕРґР° РЅР° РѕРґРЅРѕРј СЌРєСЂР°РЅРµ СЃРѕ СЃС‚РёР»СЏРјРё.", snippet: None },
    CatalogEntry { matcher: "custom_textbox", name: "Custom Textbox", description: "РљР°СЃС‚РѕРјРЅС‹Рµ С‚РµРєСЃС‚Р±РѕРєСЃС‹ СЃ РїСЂРµРґСѓСЃС‚Р°РЅРѕРІР»РµРЅРЅС‹РјРё РїРѕР»РѕР¶РµРЅРёСЏРјРё.", snippet: None },
    CatalogEntry { matcher: "textaround", name: "TextAround вЂ” РєСЂСѓРіРѕРІРѕРµ РјРµРЅСЋ", description: "РљРЅРѕРїРєРё РІС‹Р±РѕСЂРѕРІ РїРѕ РєСЂСѓРіСѓ, РІСЂР°С‰РµРЅРёРµ, С‚СЂСЏСЃРєР°, СЂР°Р·РјС‹С‚РёРµ.", snippet: None },
    CatalogEntry { matcher: "emochoice", name: "EmoChoice вЂ” СЌРјРѕС†РёРѕРЅР°Р»СЊРЅС‹Рµ РІС‹Р±РѕСЂС‹", description: "Р’Р°СЂРёР°РЅС‚С‹ РѕС‚РІРµС‚Р° СЃ СЌРјРѕС†РёРѕРЅР°Р»СЊРЅРѕР№ РѕРєСЂР°СЃРєРѕР№ Рё СЂРµР°РєС†РёРµР№ СЃРѕР±РµСЃРµРґРЅРёРєР°.", snippet: None },
    CatalogEntry { matcher: "rsp", name: "Rock-Scissors-Paper", description: "Р©РёС‚-РјРµС‡-СЃРєСЂРѕР»Р» СЃ РЅР°СЃС‚СЂР°РёРІР°РµРјРѕР№ Р»РѕРіРёРєРѕР№ Рё Р°РЅРёРјР°С†РёРµР№.", snippet: None },
    CatalogEntry { matcher: "duckhunt", name: "Duck Hunt вЂ” С‚РёСЂ", description: "РњРёРЅРё-РёРіСЂР° В«РѕС…РѕС‚Р° РЅР° СѓС‚РѕРєВ» СЃ РїРѕРјРёРґРѕСЂР°РјРё; РїРѕРїР°РґР°РЅРёРµ РїРѕ РїСЂРѕР·СЂР°С‡РЅРѕСЃС‚Рё РїРёРєСЃРµР»СЏ.", snippet: None },
    CatalogEntry { matcher: "resolutions", name: "Resolutions вЂ” СЃРјРµРЅР° СЂР°Р·СЂРµС€РµРЅРёСЏ", description: "РЎРјРµРЅР° СЂР°Р·СЂРµС€РµРЅРёСЏ РёР· quick_menu РёР»Рё СЃРїРёСЃРєР°.", snippet: None },
    CatalogEntry { matcher: "show_new_track", name: "Show New Track", description: "Р’РёРґР¶РµС‚ СЃ РЅР°Р·РІР°РЅРёРµРј С‚СЂРµРєР° РїСЂРё РєР°Р¶РґРѕРј Р·Р°РїСѓСЃРєРµ РјСѓР·С‹РєРё.", snippet: None },
    CatalogEntry { matcher: "sms", name: "SMS вЂ” С‡Р°С‚С‹", description: "РћР±С‹С‡РЅС‹Рµ Рё РіСЂСѓРїРїРѕРІС‹Рµ SMS-С‡Р°С‚С‹, СЃС‡С‘С‚С‡РёРєРё, СЌРјРѕРґР·Рё, РєР°СЂС‚РёРЅРєРё, СЃРєРёРЅС‹.", snippet: None },
    CatalogEntry { matcher: "yet_another_phone", name: "Yet Another Phone", description: "РўРµР»РµС„РѕРЅ-РјРµСЃСЃРµРЅРґР¶РµСЂ СЃ С‡Р°С‚Р°РјРё Рё СѓРІРµРґРѕРјР»РµРЅРёСЏРјРё.", snippet: None },
    CatalogEntry { matcher: "phone_messenger", name: "Phone Messenger", description: "РџРѕР»РЅРѕСЌРєСЂР°РЅРЅС‹Р№ РјРµСЃСЃРµРЅРґР¶РµСЂ РґР»СЏ РёРіСЂС‹.", snippet: None },
    CatalogEntry { matcher: "phone_number_input", name: "Phone Number Input", description: "РќР°Р±РѕСЂ РЅРѕРјРµСЂР° Рё РёРјРёС‚Р°С†РёСЏ РІС‹Р·РѕРІР°; СЂР°Р±РѕС‚Р°РµС‚ СЃ 7DOTS.", snippet: None },
    CatalogEntry { matcher: "sideimage", name: "SIW вЂ” РѕРєРЅРѕ SideImage", description: "РћРєРЅРѕ Р±РѕРєРѕРІРѕРіРѕ РёР·РѕР±СЂР°Р¶РµРЅРёСЏ СЃ СѓРґРѕР±РЅС‹Рј РІС‹СЂРµР·Р°РЅРёРµРј Рё СѓРІРµР»РёС‡РµРЅРёРµРј.", snippet: None },
    CatalogEntry { matcher: "ava", name: "Face Editor вЂ” СЂРµРґР°РєС‚РѕСЂ РіРѕР»РѕРІС‹", description: "Р¤РѕС‚РѕСЂРѕР±РѕС‚-СЂРµРґР°РєС‚РѕСЂ РїРµСЂСЃРѕРЅР°Р¶Р° РґР»СЏ SideImage.", snippet: None },
    CatalogEntry { matcher: "particle", name: "Immersive Particle VFX", description: "РЎРёСЃС‚РµРјР° С‡Р°СЃС‚РёС† РґР»СЏ Ren'Py: РёСЃРєСЂС‹, СЃРЅРµРі, РґС‹Рј Рё РґСЂСѓРіРёРµ СЌС„С„РµРєС‚С‹.", snippet: None },
    CatalogEntry { matcher: "kinetic", name: "Kinetic Text Tags", description: "РђРЅРёРјРёСЂРѕРІР°РЅРЅС‹Рµ С‚РµРєСЃС‚РѕРІС‹Рµ С‚РµРіРё: С‚РµРєСЃС‚ В«РѕР¶РёРІР°РµС‚В» РїСЂСЏРјРѕ РІ РґРёР°Р»РѕРіРµ.", snippet: None },
    CatalogEntry { matcher: "highlight", name: "Auto Highlight", description: "РђРІС‚РѕРјР°С‚РёС‡РµСЃРєР°СЏ РїРѕРґСЃРІРµС‚РєР° РІС‹РґРµР»РµРЅРЅРѕРіРѕ С‚РµРєСЃС‚Р°/СЌР»РµРјРµРЅС‚РѕРІ РёРЅС‚РµСЂС„РµР№СЃР°.", snippet: None },
    CatalogEntry { matcher: "wave", name: "Wave Rendering", description: "Р’РѕР»РЅРѕРІРѕР№ СЂРµРЅРґРµСЂРёРЅРі РёР·РѕР±СЂР°Р¶РµРЅРёР№ Рё С‚РµРєСЃС‚Р°.", snippet: None },
    CatalogEntry { matcher: "chess", name: "Ren'Py Chess", description: "РЁР°С…РјР°С‚С‹ РЅР° Ren'Py: РґРѕСЃРєР°, С„РёРіСѓСЂС‹, С…РѕРґС‹.", snippet: None },
    CatalogEntry { matcher: "dse", name: "DSE вЂ” Dating Sim Engine", description: "Р”РІРёР¶РѕРє СЃРёРјСѓР»СЏС‚РѕСЂР° СЃРІРёРґР°РЅРёР№: СЃС‚Р°С‚С‹, РєР°Р»РµРЅРґР°СЂСЊ, СЃРѕР±С‹С‚РёСЏ.", snippet: None },
    CatalogEntry { matcher: "syrup", name: "Syrup Lite RPG", description: "Р›С‘РіРєРёР№ RPG-С„СЂРµР№РјРІРѕСЂРє: Р±РѕРё, СЃС‚Р°С‚С‹, РёРЅРІРµРЅС‚Р°СЂСЊ.", snippet: None },
    CatalogEntry { matcher: "sheepstorm", name: "Sheepstorm Battle Framework", description: "РџРѕС€Р°РіРѕРІС‹Р№ Р±РѕРµРІРѕР№ С„СЂРµР№РјРІРѕСЂРє СЃ СЋРЅРёС‚Р°РјРё Рё РЅР°РІС‹РєР°РјРё.", snippet: None },
    CatalogEntry { matcher: "7dots", name: "7DOTS", description: "РќР°Р±РѕСЂ СѓС‚РёР»РёС‚, РЅР° РєРѕС‚РѕСЂРѕРј СЂР°Р±РѕС‚Р°СЋС‚ РјРЅРѕРіРёРµ РїР»Р°РіРёРЅС‹ (SMS, tasks, stereo Рё РґСЂ.).", snippet: None },
    CatalogEntry { matcher: "transitions", name: "Transitions", description: "РЎР±РѕСЂРЅРёРє РґРѕРїРѕР»РЅРёС‚РµР»СЊРЅС‹С… РїРµСЂРµС…РѕРґРѕРІ РґР»СЏ `with`.", snippet: None },
    CatalogEntry { matcher: "waste", name: "Waste Sorting", description: "РњРёРЅРё-РёРіСЂР° СЃРѕСЂС‚РёСЂРѕРІРєРё РјСѓСЃРѕСЂР° СЃ Р±Р°РєР°РјРё Рё РєР°СЂС‚РёРЅРєР°РјРё.", snippet: None },
    CatalogEntry { matcher: "points", name: "Points & Events", description: "РРіСЂРѕРІР°СЏ РІР°Р»СЋС‚Р°, С€РєР°Р»С‹ РѕС‚РЅРѕС€РµРЅРёР№ Рё РјРµРЅРµРґР¶РµСЂ СЃРѕР±С‹С‚РёР№.", snippet: None },
    CatalogEntry { matcher: "face_editor", name: "Face Editor", description: "Р РµРґР°РєС‚РѕСЂ В«РіРѕРІРѕСЂСЏС‰РµР№ РіРѕР»РѕРІС‹В» РїСЂРѕС‚Р°РіРѕРЅРёСЃС‚Р°.", snippet: None },
];

fn normalize_id(raw: &str) -> String {
    let mut value = raw.to_lowercase();
    for prefix in ["test_", "renpy-", "renpy_"] {
        value = value.trim_start_matches(prefix).to_string();
    }
    for suffix in ["-pc", "_pc", "-master", "_master"] {
        value = value.trim_end_matches(suffix).to_string();
    }
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn catalog_for(id: &str, file_name: &str) -> Option<&'static CatalogEntry> {
    let haystack = format!("{id} {file_name}").to_lowercase();
    CATALOG
        .iter()
        .filter(|entry| haystack.contains(entry.matcher))
        .max_by_key(|entry| entry.matcher.len())
}

fn clean_target(path: &str, plugin_id: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let trimmed = normalized.trim_start_matches("./");
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    if lower.contains("/cache/")
        || lower.starts_with("cache/")
        || lower.contains("/saves/")
        || lower.ends_with(".rpyc")
        || lower.ends_with(".pyc")
        || lower.contains(".git/")
        || lower.ends_with(".exe")
    {
        return None;
    }
    let relative = if let Some(rest) = trimmed.strip_prefix("game/") {
        rest.to_string()
    } else if let Some(index) = lower.find("/game/") {
        trimmed[index + 6..].to_string()
    } else if trimmed.starts_with("images/")
        || trimmed.starts_with("audio/")
        || trimmed.starts_with("fonts/")
        || trimmed.starts_with("gui/")
        || trimmed.starts_with("video/")
    {
        trimmed.to_string()
    } else if trimmed.to_lowercase().ends_with(".rpy") {
        let name = Path::new(trimmed)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "plugin.rpy".into());
        format!("plugins/{plugin_id}/{name}")
    } else if trimmed.starts_with("game") {
        trimmed.trim_start_matches("game").trim_start_matches('/').to_string()
    } else {
        return None;
    };
    if relative.is_empty()
        || relative.split('/').any(|part| part == "..")
        || relative.starts_with('/')
    {
        return None;
    }
    // Plugin archives are usually a demo game wrapped around the plugin: the
    // demo's translations and its own Ren'Py core files must never replace the
    // host project's screens, options or story entry point.
    if relative.starts_with("tl/") || relative.contains("/tl/") {
        return None;
    }
    const SCAFFOLD: [&str; 6] = [
        "script.rpy",
        "options.rpy",
        "gui.rpy",
        "screens.rpy",
        "common.rpy",
        "splashscreen.rpy",
    ];
    if SCAFFOLD.contains(&relative.as_str()) {
        return None;
    }
    Some(relative)
}

fn first_comments(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines().take(40) {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let comment = trimmed.trim_start_matches('#').trim();
            if !comment.is_empty() && !comment.starts_with('=') {
                lines.push(comment.to_string());
            }
        } else if !trimmed.is_empty() {
            break;
        }
        if lines.len() >= 3 {
            break;
        }
    }
    lines.join(" ")
}

fn read_zip_text(archive: &mut zip::ZipArchive<fs::File>, name: &str) -> Option<String> {
    let mut file = archive.by_name(name).ok()?;
    let mut text = String::new();
    file.read_to_string(&mut text).ok()?;
    Some(text)
}

fn zip_plugin(id: &str, path: &Path, meta: &fs::Metadata) -> Option<PluginInfo> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut files = Vec::new();
    let mut description = String::new();
    let mut snippet = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).ok()?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let lower = name.to_lowercase();
        if description.is_empty()
            && (lower.ends_with("readme.txt") || lower.ends_with("readme.md") || lower.ends_with("readme.rpy"))
        {
            drop(entry);
            if let Some(text) = read_zip_text(&mut archive, &name) {
                description = first_comments(&text);
            }
            continue;
        }
        if let Some(target) = clean_target(&name, id) {
            files.push(PluginFile {
                source: name,
                target,
                size: entry.size(),
            });
        }
    }
    let catalog = catalog_for(id, path.file_name()?.to_string_lossy().as_ref());
    if let Some(entry) = catalog {
        if description.is_empty() {
            description = entry.description.to_string();
        }
        snippet = entry.snippet.map(|value| value.to_string());
    }
    let size = files.iter().map(|file| file.size).sum::<u64>();
    Some(PluginInfo {
        id: id.to_string(),
        name: catalog
            .map(|entry| entry.name.to_string())
            .unwrap_or_else(|| id.replace('_', " ")),
        description,
        kind: "zip".into(),
        source: path.display().to_string(),
        size: if size == 0 { meta.len() } else { size },
        files,
        snippet,
        docs: None,
        enabled: false,
    })
}

fn folder_plugin(id: &str, path: &Path) -> Option<PluginInfo> {
    let mut files = Vec::new();
    let mut description = String::new();
    for entry in walkdir::WalkDir::new(path).max_depth(6).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(path)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        let lower = relative.to_lowercase();
        if description.is_empty() && (lower.ends_with("readme.txt") || lower.ends_with("readme.md")) {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                description = first_comments(&text);
            }
        }
        if lower.ends_with(".rpy") && description.is_empty() {
            if let Ok(text) = fs::read_to_string(entry.path()) {
                description = first_comments(&text);
            }
        }
        if let Some(target) = clean_target(&relative, id) {
            files.push(PluginFile {
                source: entry.path().display().to_string(),
                target,
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    let catalog = catalog_for(id, path.file_name()?.to_string_lossy().as_ref());
    if let Some(entry) = catalog {
        if description.is_empty() {
            description = entry.description.to_string();
        }
    }
    let size = files.iter().map(|file| file.size).sum();
    Some(PluginInfo {
        id: id.to_string(),
        name: catalog
            .map(|entry| entry.name.to_string())
            .unwrap_or_else(|| id.replace('_', " ")),
        description,
        kind: "folder".into(),
        source: path.display().to_string(),
        size,
        files,
        snippet: catalog.and_then(|entry| entry.snippet.map(|value| value.to_string())),
        docs: None,
        enabled: false,
    })
}

fn file_plugin(id: &str, path: &Path, meta: &fs::Metadata) -> Option<PluginInfo> {
    let text = fs::read_to_string(path).unwrap_or_default();
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let catalog = catalog_for(id, &file_name);
    let description = if text.is_empty() {
        catalog.map(|entry| entry.description.to_string()).unwrap_or_default()
    } else {
        first_comments(&text)
    };
    let target = format!("plugins/{id}/{file_name}");
    Some(PluginInfo {
        id: id.to_string(),
        name: catalog
            .map(|entry| entry.name.to_string())
            .unwrap_or_else(|| id.replace('_', " ")),
        description,
        kind: "file".into(),
        source: path.display().to_string(),
        size: meta.len(),
        files: vec![PluginFile {
            source: path.display().to_string(),
            target,
            size: meta.len(),
        }],
        snippet: catalog.and_then(|entry| entry.snippet.map(|value| value.to_string())),
        docs: None,
        enabled: false,
    })
}

pub fn scan(root: &Path) -> Vec<PluginInfo> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            continue;
        };
        let id = normalize_id(&stem);
        if id.is_empty() {
            continue;
        }
        let lower = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let plugin = if meta.is_dir() {
            if lower == "node_modules" || lower.starts_with('.') {
                continue;
            }
            folder_plugin(&id, &path)
        } else if lower.ends_with(".zip") {
            zip_plugin(&id, &path, &meta)
        } else if lower.ends_with(".rpy") || lower.ends_with(".rpy.txt") {
            file_plugin(&id, &path, &meta)
        } else {
            None
        };
        if let Some(plugin) = plugin {
            if !plugin.files.is_empty() {
                out.push(plugin);
            }
        }
    }
    // The same plugin often arrives as a loose .rpy and as a demo zip.
    let mut by_id: std::collections::HashMap<String, PluginInfo> = std::collections::HashMap::new();
    for plugin in out {
        match by_id.get(&plugin.id) {
            Some(existing) if existing.files.len() >= plugin.files.len() => {}
            _ => {
                by_id.insert(plugin.id.clone(), plugin);
            }
        }
    }
    let mut out: Vec<PluginInfo> = by_id.into_values().collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Copies a plugin's files into the project and returns the relative paths.
pub fn install(root: &Path, plugin: &PluginInfo) -> Result<Vec<String>, String> {
    let mut copied = Vec::new();
    match plugin.kind.as_str() {
        "zip" => {
            let file = fs::File::open(&plugin.source).map_err(|e| e.to_string())?;
            let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            for plan in &plugin.files {
                let mut entry = archive
                    .by_name(&plan.source)
                    .map_err(|e| format!("{}: {e}", plan.source))?;
                let target = safe_join(root, &plan.target)?;
                if target.exists() {
                    // Never overwrite what the author already has: plugin zips
                    // ship a demo's gui art and core screens as well as the
                    // plugin itself.
                    continue;
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut out = fs::File::create(&target).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
                copied.push(format!("game/{}", plan.target));
            }
        }
        "folder" => {
            for plan in &plugin.files {
                let source = PathBuf::from(&plan.source);
                let target = safe_join(root, &plan.target)?;
                if target.exists() {
                    continue;
                }
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                fs::copy(&source, &target).map_err(|e| {
                    format!("{} в†’ {}: {e}", source.display(), target.display())
                })?;
                copied.push(format!("game/{}", plan.target));
            }
        }
        _ => {
            let plan = plugin.files.first().ok_or("РЈ РїР»Р°РіРёРЅР° РЅРµС‚ С„Р°Р№Р»РѕРІ")?;
            let target = safe_join(root, &plan.target)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(&plan.source, &target).map_err(|e| e.to_string())?;
            copied.push(format!("game/{}", plan.target));
        }
    }
    Ok(copied)
}

pub fn uninstall(root: &Path, files: &[String]) -> Vec<String> {
    let mut removed = Vec::new();
    for relative in files {
        let trimmed = relative.trim_start_matches("game/");
        let Ok(path) = safe_join(root, trimmed) else { continue };
        if path.is_file() && fs::remove_file(&path).is_ok() {
            removed.push(relative.clone());
            // Ren'Py keeps loading orphaned .rpyc files, so a deleted plugin
            // script must take its compiled twin with it.
            if path.extension().and_then(|ext| ext.to_str()) == Some("rpy") {
                let compiled = path.with_extension("rpyc");
                if compiled.is_file() {
                    let _ = fs::remove_file(&compiled);
                }
            }
        }
    }
    removed
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let mut path = root.join("game");
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => path.push(part),
            _ => return Err("РќРµРґРѕРїСѓСЃС‚РёРјС‹Р№ РїСѓС‚СЊ РїР»Р°РіРёРЅР°".into()),
        }
    }
    Ok(path)
}

/// Where plugins live: `plugins/` next to the executable or in the workspace.
pub fn plugins_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1).take(6) {
            let candidate = ancestor.join("plugins");
            if candidate.is_dir() {
                return candidate;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("plugins");
        if candidate.is_dir() {
            return candidate;
        }
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("RenPyStudio")
        .join("plugins")
}

/// Copies a README-like file next to the plugin so the UI can show instructions.
pub fn plugin_docs(plugin: &PluginInfo) -> Option<String> {
    match plugin.kind.as_str() {
        "folder" => {
            let dir = Path::new(&plugin.source);
            for name in ["README.txt", "README.md", "readme.txt", "readme.md"] {
                let path = dir.join(name);
                if let Ok(text) = fs::read_to_string(&path) {
                    return Some(text);
                }
            }
            None
        }
        "zip" => {
            let file = fs::File::open(&plugin.source).ok()?;
            let mut archive = zip::ZipArchive::new(file).ok()?;
            for name in ["README.txt", "README.md", "readme.txt", "readme.md"] {
                for index in 0..archive.len() {
                    let entry = archive.by_index(index).ok()?;
                    if entry.name().to_lowercase().ends_with(name) {
                        let entry_name = entry.name().to_string();
                        drop(entry);
                        return read_zip_text(&mut archive, &entry_name);
                    }
                }
            }
            None
        }
        _ => fs::read_to_string(&plugin.source).ok(),
    }
}


