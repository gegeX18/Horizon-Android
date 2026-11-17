//! Upgrader (Casino+ Pack) no Horizon Android: liga `upgrader_runtime` ao jogo.
//!
//! O Upgrader na mao + clique direito abre a tela "ITEM UPGRADER" por cima da janela do
//! inventario (que serve de hospedeira para os ganchos de tela). So no mundo solo.
use super::upgrader_runtime as rt;
use crate::game::{Game, Screen};
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::NativeScreenView;
use std::time::Instant;

/// Desenha com os helpers do jogo.
struct Painter<'a, 'g> {
    game: &'a Game<'g>,
    batch: &'a mut HudBatch,
}

impl rt::Draw for Painter<'_, '_> {
    fn fill(&mut self, rect: [f32; 4], argb: u32) {
        self.game.native_fill(self.batch, rect, argb);
    }
    fn quad(&mut self, points: [[f32; 2]; 4], argb: u32) {
        let (game, batch) = (self.game, &mut *self.batch);
        rt::raster_quad(points, argb, &mut |r, c| game.native_fill(batch, r, c));
    }
    fn text(&mut self, s: &str, at: [f32; 2], rgb: u32, shadow: bool) {
        self.game.native_text(self.batch, s, at, rgb, shadow);
    }
    fn text_width(&self, s: &str) -> f32 {
        self.game.native_text_width(s)
    }
    fn item(&mut self, id: &str, at: [f32; 2], size: f32) -> bool {
        self.game.native_item(self.batch, id, at, size)
    }
}

/// O inventario do jogador local.
struct GameInv<'a, 'g>(&'a mut Game<'g>);

impl rt::Inv for GameInv<'_, '_> {
    fn count(&mut self, item: &str) -> u32 {
        self.0.native_count_items(item)
    }
    fn take(&mut self, item: &str, n: u32) -> u32 {
        self.0.native_take_items(item, n)
    }
    fn room(&mut self, item: &str) -> u32 {
        self.0.native_room_for(item)
    }
    fn give(&mut self, item: &str, n: u32) -> u32 {
        self.0.native_give_items(item, n)
    }
}

/// O jogador usou o Upgrader: pede para abrir a tela e segura o uso normal do item.
pub(super) fn on_use_item(stack: &ItemStack, claimed: &mut bool) {
    if &*stack.item == rt::ITEM {
        rt::request_open();
        *claimed = true;
    }
}

/// Uma vez por tick, so na janela do jogo solo: abre a tela, gasta a entrada e entrega o premio.
pub(super) fn tick(game: &mut Game<'_>) {
    if game.renderer.is_none() {
        return;
    }
    if game.remote.is_some() || game.player.is_none() {
        rt::clear_open_request();
        return;
    }
    if rt::open_requested() {
        rt::clear_open_request();
        if game.screen == Screen::Playing {
            game.screen = Screen::Inventory;
            game.capture(false);
            rt::on_open();
            rt::set_active(true);
        }
    }
    if rt::is_active() && game.screen != Screen::Inventory {
        rt::set_active(false);
    }
    let mut ui = rt::ui();
    if ui.spin.is_none() {
        return;
    }
    rt::tick(&mut ui, &mut GameInv(game), Instant::now());
}

fn upgrader_slots(game: &Game<'_>) -> Vec<Option<(String, u32)>> {
    (0..36)
        .map(|i| {
            game.inventory
                .get(i)
                .and_then(Option::as_ref)
                .map(|s| (s.item.to_string(), s.count))
        })
        .collect()
}

/// Desenha a tela por cima do inventario (gancho `native_screen_render_post`).
pub(super) fn render(game: &Game<'_>, batch: &mut HudBatch, screen: &NativeScreenView<'_>) {
    if !rt::is_active() || screen.id != "inventory" {
        return;
    }
    rt::set_gui_size(batch.width, batch.height);
    let lay = rt::Layout::new(batch.width, batch.height);
    let slots = upgrader_slots(game);
    let carried = game.native_carried_items();
    let ui = rt::ui();
    let mut painter = Painter { game, batch };
    rt::draw(&ui, &lay, &mut painter, screen.mouse, &slots, &carried, Instant::now());
}

/// Clique do mouse ou toque (gancho `native_screen_mouse_button_pressed_pre`).
pub(super) fn click(
    game: &Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if !rt::is_active() || screen.id != "inventory" {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    // A tela ocupa a janela toda; o tamanho e o do ultimo quadro desenhado.
    let size = rt::gui_size();
    if size[0] <= 0.0 || size[1] <= 0.0 {
        return;
    }
    let lay = rt::Layout::new(size[0], size[1]);
    let pos = screen.mouse;
    let slot_item = lay.slot_at(pos).map(|i| {
        game.inventory
            .get(i)
            .and_then(Option::as_ref)
            .map(|s| s.item.to_string())
    });
    let carried = game.native_carried_items();
    let mut ui = rt::ui();
    rt::click(&mut ui, &lay, pos, slot_item, &carried, Instant::now());
}
