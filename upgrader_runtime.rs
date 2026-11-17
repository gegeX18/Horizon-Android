//! Upgrader (Casino+ Pack) para o Horizon Android: valores, chance, roleta, estado, layout e desenho.
//! Nenhum tipo do jogo aparece aqui; `upgrader_native.rs` liga isto ao jogo.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

pub const ITEM: &str = "upgrader:upgrader";

pub const HOUSE_EDGE: f64 = 0.95;
pub const MIN_CHANCE: f64 = 0.0;
pub const MAX_CHANCE: f64 = 0.9;
pub const MAX_TARGET: u32 = 64;
pub const SPIN_MS: u64 = 3500;

// ---------------------------------------------------------------- valores

const BLACKLIST: [&str; 10] = [
    "minecraft:bedrock",
    "minecraft:barrier",
    "minecraft:command_block",
    "minecraft:chain_command_block",
    "minecraft:repeating_command_block",
    "minecraft:structure_block",
    "minecraft:jigsaw",
    "minecraft:debug_stick",
    "minecraft:light",
    "upgrader:upgrader",
];

/// Valores do Casino+ Pack (`valueOverrides`).
fn base(id: &str) -> Option<f64> {
    Some(match id {
        "minecraft:coal" | "minecraft:charcoal" | "minecraft:redstone" => 1.0,
        "minecraft:copper_ingot" | "minecraft:lapis_lazuli" => 2.0,
        "minecraft:amethyst_shard" | "minecraft:quartz" => 4.0,
        "minecraft:iron_ingot" => 8.0,
        "minecraft:gold_ingot" => 12.0,
        "minecraft:emerald" => 32.0,
        "minecraft:diamond" => 64.0,
        "minecraft:ancient_debris" | "minecraft:netherite_scrap" => 128.0,
        "minecraft:netherite_ingot" => 560.0,
        _ => return None,
    })
}

/// Valor de um item. A tabela embutida do mod original nao e conhecida: alem dos
/// valores do pack, blocos de material valem 9x (ou 4x), ferramentas e armaduras
/// valem o material gasto na receita, e todo o resto vale 1.
pub fn value_of(id: &str) -> Option<f64> {
    if BLACKLIST.contains(&id) {
        return None;
    }
    if let Some(v) = base(id) {
        return Some(v);
    }
    let Some(name) = id.strip_prefix("minecraft:") else {
        return Some(1.0);
    };
    let block = match name {
        "iron_block" => Some(("minecraft:iron_ingot", 9.0)),
        "gold_block" => Some(("minecraft:gold_ingot", 9.0)),
        "copper_block" => Some(("minecraft:copper_ingot", 9.0)),
        "diamond_block" => Some(("minecraft:diamond", 9.0)),
        "emerald_block" => Some(("minecraft:emerald", 9.0)),
        "lapis_block" => Some(("minecraft:lapis_lazuli", 9.0)),
        "redstone_block" => Some(("minecraft:redstone", 9.0)),
        "coal_block" => Some(("minecraft:coal", 9.0)),
        "netherite_block" => Some(("minecraft:netherite_ingot", 9.0)),
        "amethyst_block" => Some(("minecraft:amethyst_shard", 4.0)),
        "quartz_block" => Some(("minecraft:quartz", 4.0)),
        _ => None,
    };
    if let Some((src, n)) = block {
        return base(src).map(|v| v * n);
    }
    for (piece, n) in [
        ("sword", 2.0),
        ("pickaxe", 3.0),
        ("axe", 3.0),
        ("shovel", 1.0),
        ("hoe", 2.0),
        ("helmet", 5.0),
        ("chestplate", 8.0),
        ("leggings", 7.0),
        ("boots", 4.0),
    ] {
        if let Some(mat) = name.strip_suffix(piece).and_then(|m| m.strip_suffix('_')) {
            let unit = match mat {
                "iron" => base("minecraft:iron_ingot"),
                "gold" => base("minecraft:gold_ingot"),
                "diamond" => base("minecraft:diamond"),
                "netherite" => {
                    return Some(base("minecraft:diamond")? * n + base("minecraft:netherite_ingot")?)
                }
                _ => None,
            };
            if let Some(u) = unit {
                return Some(u * n);
            }
        }
    }
    Some(1.0)
}

/// Itens do catalogo (alvos possiveis).
pub const CATALOGUE: &[&str] = &[
    "minecraft:coal", "minecraft:redstone", "minecraft:copper_ingot", "minecraft:lapis_lazuli",
    "minecraft:amethyst_shard", "minecraft:quartz", "minecraft:iron_ingot", "minecraft:gold_ingot",
    "minecraft:emerald", "minecraft:diamond", "minecraft:netherite_scrap", "minecraft:netherite_ingot",
    "minecraft:iron_block", "minecraft:gold_block", "minecraft:diamond_block", "minecraft:emerald_block",
    "minecraft:netherite_block", "minecraft:ancient_debris", "minecraft:iron_sword", "minecraft:iron_pickaxe",
    "minecraft:iron_axe", "minecraft:iron_shovel", "minecraft:iron_helmet", "minecraft:iron_chestplate",
    "minecraft:iron_leggings", "minecraft:iron_boots", "minecraft:gold_sword", "minecraft:gold_pickaxe",
    "minecraft:gold_helmet", "minecraft:gold_chestplate", "minecraft:gold_leggings", "minecraft:gold_boots",
    "minecraft:diamond_sword", "minecraft:diamond_pickaxe", "minecraft:diamond_axe", "minecraft:diamond_shovel",
    "minecraft:diamond_helmet", "minecraft:diamond_chestplate", "minecraft:diamond_leggings", "minecraft:diamond_boots",
    "minecraft:netherite_sword", "minecraft:netherite_pickaxe", "minecraft:netherite_axe", "minecraft:netherite_shovel",
    "minecraft:netherite_helmet", "minecraft:netherite_chestplate", "minecraft:netherite_leggings", "minecraft:netherite_boots",
    "minecraft:ender_pearl", "minecraft:blaze_rod", "minecraft:golden_apple", "minecraft:totem_of_undying",
    "minecraft:elytra", "minecraft:nether_star", "minecraft:obsidian", "minecraft:glowstone_dust",
];

pub fn pretty(id: &str) -> String {
    let name = id.rsplit(':').next().unwrap_or(id).replace('_', " ");
    let mut c = name.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn fmt_value(v: f64) -> String {
    if v >= 1000.0 {
        format!("{:.1}K", v / 1000.0)
    } else {
        format!("{}", v.round() as i64)
    }
}

// ----------------------------------------------------------------- chance

#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    pub in_count: u32,
    pub in_value: f64,
    pub unit: f64,
    pub target_value: f64,
    pub chance: f64,
}

pub fn chance_of(in_value: f64, target_value: f64) -> f64 {
    if target_value <= 0.0 {
        return 0.0;
    }
    (in_value / target_value * HOUSE_EDGE).clamp(MIN_CHANCE, MAX_CHANCE)
}

pub fn amount_for_multiple(in_value: f64, unit: f64, mult: f64) -> u32 {
    ((in_value * mult / unit).round() as i64).clamp(1, MAX_TARGET as i64) as u32
}

pub fn amount_for_chance(in_value: f64, unit: f64, p: f64) -> u32 {
    ((in_value * HOUSE_EDGE / (p * unit)).round() as i64).clamp(1, MAX_TARGET as i64) as u32
}

static CTR: AtomicU64 = AtomicU64::new(0);

/// Aleatorio uniforme em [0, 1) (SplitMix64 semeado pelo relogio e por um contador).
pub fn roll() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x1234_5678);
    let mut z = t
        .wrapping_add(CTR.fetch_add(0x9E37_79B9_7F4A_7C15, Ordering::Relaxed))
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

// ----------------------------------------------------------------- estado

#[derive(Clone, Debug)]
pub struct SpinState {
    pub started: Instant,
    pub final_deg: f64,
    pub win: bool,
    pub chance: f64,
    pub input: String,
    pub in_count: u32,
    pub target: String,
    pub amount: u32,
    pub taken: bool,
    pub granted: bool,
}

pub struct Ui {
    pub input: Option<String>,
    pub target: Option<String>,
    pub amount: u32,
    pub catalogue: bool,
    pub spin: Option<SpinState>,
    pub msg: String,
}

impl Ui {
    pub const fn new() -> Ui {
        Ui { input: None, target: None, amount: 1, catalogue: false, spin: None, msg: String::new() }
    }
}

pub static UI: Mutex<Ui> = Mutex::new(Ui::new());

static OPEN: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// A tela do Upgrader esta aberta (por cima da janela do inventario).
pub fn set_active(on: bool) {
    ACTIVE.store(on, Ordering::Relaxed);
}

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

static GUI_W: AtomicU64 = AtomicU64::new(0);
static GUI_H: AtomicU64 = AtomicU64::new(0);

/// Tamanho da GUI (em pixels de GUI) usado no ultimo quadro desenhado; o clique usa o mesmo.
pub fn set_gui_size(w: f32, h: f32) {
    GUI_W.store(w.to_bits() as u64, Ordering::Relaxed);
    GUI_H.store(h.to_bits() as u64, Ordering::Relaxed);
}

pub fn gui_size() -> [f32; 2] {
    [f32::from_bits(GUI_W.load(Ordering::Relaxed) as u32), f32::from_bits(GUI_H.load(Ordering::Relaxed) as u32)]
}

/// O jogador usou o Upgrader: a janela do jogo abre a tela no proximo tick.
pub fn request_open() {
    OPEN.store(true, Ordering::Relaxed);
}

pub fn open_requested() -> bool {
    OPEN.load(Ordering::Relaxed)
}

pub fn clear_open_request() {
    OPEN.store(false, Ordering::Relaxed);
}

pub fn ui() -> MutexGuard<'static, Ui> {
    UI.lock().unwrap_or_else(|e| e.into_inner())
}

/// Chamado quando a tela abre: limpa a escolha (o inventario pode ter mudado).
pub fn on_open() {
    let mut u = ui();
    let busy = u.spin.as_ref().is_some_and(|s| !s.granted);
    u.catalogue = false;
    if !busy {
        u.input = None;
        u.spin = None;
        u.msg.clear();
    }
}

pub fn quote(ui: &Ui, carried: &[(String, u32)]) -> Option<Quote> {
    let input = ui.input.as_deref()?;
    let target = ui.target.as_deref()?;
    let in_count = carried.iter().find(|(i, _)| i == input).map(|(_, n)| *n).unwrap_or(0);
    let unit = value_of(target)?;
    let in_value = value_of(input)? * in_count as f64;
    let target_value = unit * ui.amount as f64;
    Some(Quote { in_count, in_value, unit, target_value, chance: chance_of(in_value, target_value) })
}

pub fn busy(ui: &Ui) -> bool {
    ui.spin.as_ref().is_some_and(|s| !s.granted)
}

pub fn can_upgrade(ui: &Ui, q: &Option<Quote>) -> bool {
    !busy(ui) && q.as_ref().is_some_and(|q| q.in_count > 0 && q.chance > 0.0)
}

pub fn start_spin(ui: &mut Ui, q: &Quote, now: Instant) {
    let (Some(input), Some(target)) = (ui.input.clone(), ui.target.clone()) else { return };
    let win = roll() < q.chance;
    let zone = q.chance * 360.0;
    let r = roll();
    let final_deg = if win {
        zone * (0.04 + 0.92 * r)
    } else {
        zone + (360.0 - zone) * (0.04 + 0.92 * r)
    };
    ui.msg.clear();
    ui.spin = Some(SpinState {
        started: now,
        final_deg,
        win,
        chance: q.chance,
        input,
        in_count: q.in_count,
        target,
        amount: ui.amount,
        taken: false,
        granted: false,
    });
}

/// O inventario do jogador, como o mod o enxerga.
pub trait Inv {
    fn count(&mut self, item: &str) -> u32;
    fn take(&mut self, item: &str, n: u32) -> u32;
    fn room(&mut self, item: &str) -> u32;
    fn give(&mut self, item: &str, n: u32) -> u32;
}

/// Uma vez por tick: gasta a entrada no inicio do giro e entrega o premio no fim,
/// mesmo que a tela tenha sido fechada. O resultado ja foi decidido ao clicar.
pub fn tick(ui: &mut Ui, inv: &mut dyn Inv, now: Instant) {
    let Some(s) = ui.spin.as_mut() else { return };
    if !s.taken {
        if inv.count(&s.input) < s.in_count {
            ui.msg = "Not enough items.".into();
            ui.spin = None;
            return;
        }
        if s.win && inv.room(&s.target) < s.amount {
            ui.msg = "No room in your inventory.".into();
            ui.spin = None;
            return;
        }
        let took = inv.take(&s.input, s.in_count);
        if took < s.in_count {
            inv.give(&s.input, took);
            ui.msg = "Could not take the items.".into();
            ui.spin = None;
            return;
        }
        s.taken = true;
    }
    if !s.granted && now.duration_since(s.started).as_millis() as u64 >= SPIN_MS {
        if s.win {
            inv.give(&s.target, s.amount);
        }
        s.granted = true;
    }
}

// ----------------------------------------------------------------- layout

#[derive(Clone, Copy, Debug)]
pub struct R {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl R {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> R {
        R { x, y, w, h }
    }
    pub fn arr(&self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }
    pub fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.x && p[0] < self.x + self.w && p[1] >= self.y && p[1] < self.y + self.h
    }
}

pub struct Layout {
    pub screen: R,
    pub panel: R,
    pub input: R,
    pub target: R,
    pub center: [f32; 2],
    pub r_out: f32,
    pub r_in: f32,
    pub upgrade: R,
    pub quick: [R; 6],
    pub minus: R,
    pub plus: R,
    pub inv_x: f32,
    pub inv_y: f32,
    pub label_y: f32,
    pub cat: R,
    pub cat_close: R,
    pub cat_x: f32,
    pub cat_y: f32,
}

pub const SLOT: f32 = 18.0;
pub const CAT_COLS: usize = 10;

impl Layout {
    pub fn new(w: f32, h: f32) -> Layout {
        let pw = (w - 4.0).min(380.0);
        let ph = (h - 4.0).min(232.0);
        let px = (w - pw) / 2.0;
        let py = (h - ph) / 2.0;
        let side = (pw - 24.0 - 88.0) / 2.0;
        let row_a = py + 22.0;
        let input = R::new(px + 8.0, row_a, side, 88.0);
        let target = R::new(px + pw - 8.0 - side, row_a, side, 88.0);
        let center = [px + pw / 2.0, row_a + 44.0];
        let row_b = py + 114.0;
        let upgrade = R::new(px + 8.0, row_b, 100.0, 16.0);
        let rem = pw - 16.0 - 100.0 - 6.0;
        let qw = (rem - 5.0 * 2.0) / 6.0;
        let mut quick = [R::new(0.0, 0.0, 0.0, 0.0); 6];
        for (i, q) in quick.iter_mut().enumerate() {
            *q = R::new(px + 8.0 + 106.0 + i as f32 * (qw + 2.0), row_b, qw, 16.0);
        }
        let minus = R::new(target.x + 6.0, target.y + target.h - 20.0, 16.0, 14.0);
        let plus = R::new(target.x + target.w - 22.0, target.y + target.h - 20.0, 16.0, 14.0);
        let cat = R::new(px + (pw - 200.0) / 2.0, py + 14.0, 200.0, 190.0);
        Layout {
            screen: R::new(0.0, 0.0, w, h),
            panel: R::new(px, py, pw, ph),
            input,
            target,
            center,
            r_out: 44.0,
            r_in: 33.0,
            upgrade,
            quick,
            minus,
            plus,
            inv_x: px + (pw - 9.0 * SLOT) / 2.0,
            inv_y: py + 146.0,
            label_y: py + 134.0,
            cat,
            cat_close: R::new(cat.x + cat.w - 16.0, cat.y + 3.0, 13.0, 13.0),
            cat_x: cat.x + 10.0,
            cat_y: cat.y + 22.0,
        }
    }

    /// Indice (0..36, como o jogo numera: 0 a 8 barra, 9 a 35 mochila) do slot sob `p`.
    pub fn slot_at(&self, p: [f32; 2]) -> Option<usize> {
        let (dx, dy) = (p[0] - self.inv_x, p[1] - self.inv_y);
        if dx < 0.0 || dy < 0.0 {
            return None;
        }
        let (col, row) = ((dx / SLOT) as usize, (dy / SLOT) as usize);
        if col >= 9 || row >= 4 {
            return None;
        }
        Some(if row == 3 { col } else { 9 + row * 9 + col })
    }

    pub fn slot_origin(&self, idx: usize) -> [f32; 2] {
        let (row, col) = if idx < 9 { (3, idx) } else { ((idx - 9) / 9, (idx - 9) % 9) };
        [self.inv_x + col as f32 * SLOT, self.inv_y + row as f32 * SLOT]
    }

    pub fn cat_index(&self, p: [f32; 2], len: usize) -> Option<usize> {
        let (dx, dy) = (p[0] - self.cat_x, p[1] - self.cat_y);
        if dx < 0.0 || dy < 0.0 {
            return None;
        }
        let (col, row) = ((dx / SLOT) as usize, (dy / SLOT) as usize);
        let i = row * CAT_COLS + col;
        (col < CAT_COLS && i < len).then_some(i)
    }
}

// -------------------------------------------------------------- geometria

/// Ponto a `r` do centro, `deg` graus no sentido horario a partir do topo.
pub fn polar(c: [f32; 2], r: f32, deg: f32) -> [f32; 2] {
    let a = (deg - 90.0).to_radians();
    [c[0] + r * a.cos(), c[1] + r * a.sin()]
}

pub fn wedge(c: [f32; 2], r_in: f32, r_out: f32, d0: f32, d1: f32) -> [[f32; 2]; 4] {
    [polar(c, r_out, d0), polar(c, r_out, d1), polar(c, r_in, d1), polar(c, r_in, d0)]
}

pub fn bar(c: [f32; 2], r0: f32, r1: f32, deg: f32, half: f32) -> [[f32; 2]; 4] {
    let a = (deg - 90.0).to_radians();
    let n = [-a.sin() * half, a.cos() * half];
    let (p0, p1) = (polar(c, r0, deg), polar(c, r1, deg));
    [[p0[0] + n[0], p0[1] + n[1]], [p1[0] + n[0], p1[1] + n[1]], [p1[0] - n[0], p1[1] - n[1]], [p0[0] - n[0], p0[1] - n[1]]]
}

/// Angulo da agulha: 3 voltas freando ate parar em `final_deg`.
pub fn needle_deg(t: f64, final_deg: f64) -> f64 {
    let e = 1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3);
    e * (1080.0 + final_deg)
}

/// Preenche um quadrilatero convexo com retangulos de 1 pixel de altura: esta versao
/// do jogo nao tem helper de quadrilatero.
pub fn raster_quad(p: [[f32; 2]; 4], argb: u32, put: &mut dyn FnMut([f32; 4], u32)) {
    let (mut y0, mut y1) = (f32::MAX, f32::MIN);
    for q in &p {
        y0 = y0.min(q[1]);
        y1 = y1.max(q[1]);
    }
    for y in (y0.floor() as i32)..(y1.ceil() as i32) {
        let yc = y as f32 + 0.5;
        let (mut xl, mut xr) = (f32::MAX, f32::MIN);
        for i in 0..4 {
            let (a, b) = (p[i], p[(i + 1) % 4]);
            if (a[1] <= yc && b[1] > yc) || (b[1] <= yc && a[1] > yc) {
                let t = (yc - a[1]) / (b[1] - a[1]);
                let x = a[0] + t * (b[0] - a[0]);
                xl = xl.min(x);
                xr = xr.max(x);
            }
        }
        if xl < xr {
            let l = xl.round();
            put([l, y as f32, (xr.round() - l).max(1.0), 1.0], argb);
        }
    }
}

// ----------------------------------------------------------------- desenho

pub trait Draw {
    fn fill(&mut self, rect: [f32; 4], argb: u32);
    fn quad(&mut self, points: [[f32; 2]; 4], argb: u32);
    fn text(&mut self, s: &str, at: [f32; 2], rgb: u32, shadow: bool);
    fn text_width(&self, s: &str) -> f32;
    fn item(&mut self, id: &str, at: [f32; 2], size: f32) -> bool;
}

fn centered(d: &mut dyn Draw, s: &str, cx: f32, y: f32, rgb: u32) {
    let w = d.text_width(s);
    d.text(s, [cx - w / 2.0, y], rgb, true);
}

fn frame(d: &mut dyn Draw, r: R, fill: u32, border: u32) {
    d.fill(r.arr(), border);
    d.fill([r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0], fill);
}

fn button(d: &mut dyn Draw, r: R, label: &str, enabled: bool, hover: bool, gold: bool) {
    let (fill, text) = match (enabled, gold) {
        (false, _) => (0xFF222633, 0x6C7184),
        (true, true) => (if hover { 0xFFFFC94D } else { 0xFFE5A92B }, 0x2A1A00),
        (true, false) => (if hover { 0xFF3A4054 } else { 0xFF2C3142 }, 0xD8DCEB),
    };
    frame(d, r, fill, 0xFF0C0E14);
    centered(d, label, r.x + r.w / 2.0, r.y + (r.h - 8.0) / 2.0, text);
}

pub fn draw(ui: &Ui, lay: &Layout, d: &mut dyn Draw, mouse: [f32; 2], slots: &[Option<(String, u32)>], carried: &[(String, u32)], now: Instant) {
    let q = quote(ui, carried);
    d.fill(lay.screen.arr(), 0xFF0A0C12);
    frame(d, lay.panel, 0xF0151822, 0xFF3A3F52);
    centered(d, "ITEM UPGRADER", lay.panel.x + lay.panel.w / 2.0, lay.panel.y + 4.0, 0xF2D27A);

    // painel da entrada
    frame(d, lay.input, 0xFF10131C, 0xFF3A3F52);
    let icx = lay.input.x + lay.input.w / 2.0;
    match (&ui.input, &q) {
        (Some(id), Some(q)) => {
            d.item(id, [icx - 12.0, lay.input.y + 8.0], 24.0);
            centered(d, &format!("x{}", q.in_count), icx, lay.input.y + 34.0, 0xFFFFFF);
            centered(d, &pretty(id), icx, lay.input.y + 48.0, 0xD8DCEB);
            centered(d, &format!("Value: {}", fmt_value(q.in_value)), icx, lay.input.y + 62.0, 0x9AA0B4);
        }
        (Some(id), None) => {
            d.item(id, [icx - 12.0, lay.input.y + 8.0], 24.0);
            centered(d, &pretty(id), icx, lay.input.y + 48.0, 0xD8DCEB);
            centered(d, "Choose the target", icx, lay.input.y + 62.0, 0x9AA0B4);
        }
        _ => {
            centered(d, "Choose an item", icx, lay.input.y + 26.0, 0xFFFFFF);
            centered(d, "to upgrade", icx, lay.input.y + 37.0, 0xFFFFFF);
            centered(d, "Tap it in your", icx, lay.input.y + 54.0, 0x8A8FA0);
            centered(d, "inventory", icx, lay.input.y + 65.0, 0x8A8FA0);
        }
    }

    // painel do alvo
    frame(d, lay.target, 0xFF10131C, 0xFF3A3F52);
    let tcx = lay.target.x + lay.target.w / 2.0;
    match &ui.target {
        Some(id) => {
            d.item(id, [tcx - 12.0, lay.target.y + 6.0], 24.0);
            centered(d, &pretty(id), tcx, lay.target.y + 33.0, 0xD8DCEB);
            let v = value_of(id).unwrap_or(0.0) * ui.amount as f64;
            centered(d, &format!("Value: {}", fmt_value(v)), tcx, lay.target.y + 45.0, 0x9AA0B4);
            centered(d, "Amount", tcx, lay.target.y + 57.0, 0x9AA0B4);
            button(d, lay.minus, "-", true, lay.minus.contains(mouse), false);
            button(d, lay.plus, "+", true, lay.plus.contains(mouse), false);
            centered(d, &format!("{}", ui.amount), tcx, lay.target.y + 69.0, 0xFFD24A);
        }
        None => {
            centered(d, "Choose the item", tcx, lay.target.y + 26.0, 0xFFFFFF);
            centered(d, "you want", tcx, lay.target.y + 37.0, 0xFFFFFF);
            centered(d, "Tap to open", tcx, lay.target.y + 54.0, 0x8A8FA0);
            centered(d, "the catalogue", tcx, lay.target.y + 65.0, 0x8A8FA0);
        }
    }

    // roleta
    let c = lay.center;
    let (chance, label, color) = match (&ui.spin, &q) {
        (Some(s), _) => {
            let done = s.granted || now.duration_since(s.started).as_millis() as u64 >= SPIN_MS;
            if !done {
                (s.chance, "spinning...", 0xFFE24A)
            } else if s.win {
                (s.chance, "SUCCESS!", 0x5CE05C)
            } else {
                (s.chance, "FAILED", 0xE04848)
            }
        }
        (None, Some(q)) if q.chance > 0.0 => (q.chance, "chance", 0x7CD65C),
        _ => (0.0, "chance", 0xE04848),
    };
    let zone = chance * 360.0;
    for i in 0..90 {
        let d0 = i as f32 * 4.0;
        let col = if (d0 as f64) < zone { 0xFFF29A2E } else { 0xFF262A36 };
        d.quad(wedge(c, lay.r_in, lay.r_out, d0, d0 + 4.3), col);
    }
    for i in 0..36 {
        let big = i % 3 == 0;
        d.quad(bar(c, lay.r_out + 2.0, lay.r_out + if big { 6.0 } else { 4.0 }, i as f32 * 10.0, 0.6), 0xFF6C7184);
    }
    let deg = match &ui.spin {
        Some(s) => needle_deg(now.duration_since(s.started).as_millis() as f64 / SPIN_MS as f64, s.final_deg),
        None => 0.0,
    };
    d.quad(bar(c, lay.r_in - 6.0, lay.r_out + 3.0, deg as f32, 1.1), 0xFFFFFFFF);
    d.quad(
        [[c[0] - 4.0, c[1] - lay.r_out - 9.0], [c[0] + 4.0, c[1] - lay.r_out - 9.0], [c[0], c[1] - lay.r_out - 3.0], [c[0], c[1] - lay.r_out - 3.0]],
        0xFFFFD24A,
    );
    centered(d, &format!("{:.2}%", chance * 100.0), c[0], c[1] - 9.0, color);
    centered(d, label, c[0], c[1] + 3.0, if label == "chance" { 0x9AA0B4 } else { color });

    // botoes
    let ready = can_upgrade(ui, &q);
    button(d, lay.upgrade, "UPGRADE", ready, lay.upgrade.contains(mouse), true);
    let labels = ["x2", "x4", "x8", "30%", "50%", "70%"];
    let have = q.is_some();
    for (r, l) in lay.quick.iter().zip(labels) {
        button(d, *r, l, have, r.contains(mouse), false);
    }
    d.text("Inventário", [lay.inv_x, lay.label_y], 0x8A8FA0, false);
    if !ui.msg.is_empty() {
        let w = d.text_width(&ui.msg);
        d.text(&ui.msg, [lay.inv_x + 9.0 * SLOT - w, lay.label_y], 0xE0A040, false);
    }

    // inventario
    for idx in 0..36 {
        let at = lay.slot_origin(idx);
        let held = slots.get(idx).and_then(|s| s.as_ref());
        let picked = held.is_some_and(|(id, _)| ui.input.as_deref() == Some(id.as_str()));
        let hov = R::new(at[0], at[1], SLOT, SLOT).contains(mouse);
        frame(d, R::new(at[0], at[1], SLOT, SLOT), if hov { 0xFF2A3042 } else { 0xFF171A24 }, if picked { 0xFFE5A92B } else { 0xFF2B2F3D });
        if let Some((id, n)) = held {
            d.item(id, [at[0] + 1.0, at[1] + 1.0], 16.0);
            if *n > 1 {
                let s = format!("{n}");
                let w = d.text_width(&s);
                d.text(&s, [at[0] + SLOT - 1.0 - w, at[1] + SLOT - 9.0], 0xFFFFFF, true);
            }
        }
    }

    // catalogo
    if ui.catalogue {
        d.fill(lay.screen.arr(), 0x80000000);
        frame(d, lay.cat, 0xFF151822, 0xFF3A3F52);
        d.text("Catalogue", [lay.cat.x + 8.0, lay.cat.y + 5.0], 0xF2D27A, true);
        button(d, lay.cat_close, "x", true, lay.cat_close.contains(mouse), false);
        for (i, id) in CATALOGUE.iter().enumerate() {
            let at = [lay.cat_x + (i % CAT_COLS) as f32 * SLOT, lay.cat_y + (i / CAT_COLS) as f32 * SLOT];
            let hov = R::new(at[0], at[1], SLOT, SLOT).contains(mouse);
            let sel = ui.target.as_deref() == Some(*id);
            frame(d, R::new(at[0], at[1], SLOT, SLOT), if hov { 0xFF2A3042 } else { 0xFF171A24 }, if sel { 0xFFE5A92B } else { 0xFF2B2F3D });
            d.item(id, [at[0] + 1.0, at[1] + 1.0], 16.0);
        }
        if let Some(i) = lay.cat_index(mouse, CATALOGUE.len()) {
            let id = CATALOGUE[i];
            let t = format!("{}  (Value: {})", pretty(id), fmt_value(value_of(id).unwrap_or(0.0)));
            d.text(&t, [lay.cat.x + 8.0, lay.cat.y + lay.cat.h - 12.0], 0xD8DCEB, false);
        }
    }
}

// ------------------------------------------------------------------ cliques

/// Trata um clique esquerdo na tela. `slot_item`: None se `pos` nao esta num slot,
/// Some(None) num slot vazio, Some(Some(id)) num slot com item.
pub fn click(ui: &mut Ui, lay: &Layout, pos: [f32; 2], slot_item: Option<Option<String>>, carried: &[(String, u32)], now: Instant) {
    if ui.catalogue {
        if lay.cat_close.contains(pos) || !lay.cat.contains(pos) {
            ui.catalogue = false;
        } else if let Some(i) = lay.cat_index(pos, CATALOGUE.len()) {
            ui.target = Some(CATALOGUE[i].to_string());
            ui.amount = ui.amount.clamp(1, MAX_TARGET);
            ui.catalogue = false;
            if !busy(ui) {
                ui.spin = None;
            }
            ui.msg.clear();
        }
        return;
    }
    if busy(ui) {
        return;
    }
    let q = quote(ui, carried);
    if ui.target.is_some() && lay.minus.contains(pos) {
        ui.amount = ui.amount.saturating_sub(1).max(1);
        ui.spin = None;
    } else if ui.target.is_some() && lay.plus.contains(pos) {
        ui.amount = (ui.amount + 1).min(MAX_TARGET);
        ui.spin = None;
    } else if lay.target.contains(pos) {
        ui.catalogue = true;
    } else if lay.upgrade.contains(pos) {
        if let (true, Some(q)) = (can_upgrade(ui, &q), q.as_ref()) {
            start_spin(ui, q, now);
        }
    } else if let Some(i) = lay.quick.iter().position(|r| r.contains(pos)) {
        if let Some(q) = q.as_ref() {
            ui.amount = match i {
                0 => amount_for_multiple(q.in_value, q.unit, 2.0),
                1 => amount_for_multiple(q.in_value, q.unit, 4.0),
                2 => amount_for_multiple(q.in_value, q.unit, 8.0),
                3 => amount_for_chance(q.in_value, q.unit, 0.3),
                4 => amount_for_chance(q.in_value, q.unit, 0.5),
                _ => amount_for_chance(q.in_value, q.unit, 0.7),
            };
            ui.spin = None;
        }
    } else if let Some(slot) = slot_item {
        match slot {
            Some(id) if value_of(&id).is_none() => ui.msg = "That item can't be upgraded.".into(),
            Some(id) if !carried.iter().any(|(i, _)| *i == id) => ui.msg = "Damaged or enchanted items aren't accepted.".into(),
            Some(id) => {
                ui.input = Some(id);
                ui.spin = None;
                ui.msg.clear();
            }
            None => {}
        }
    }
}
