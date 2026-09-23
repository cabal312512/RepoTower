use crate::{
    app::{App, Popup},
    graph::View,
    paint::*,
    text::{self, tr},
};
use eframe::egui::*;

impl App {
    fn menu_item(&mut self, ui: &mut Ui, id: &str, caption: &str, enabled: bool) -> bool {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 29.0), Sense::hover());
        let p = self.palette();
        button(
            ui,
            &mut self.controls,
            id,
            rect,
            caption,
            None,
            p,
            false,
            enabled,
            false,
        )
        .clicked()
    }
    pub fn popup_ui(&mut self, ctx: &Context, screen: Rect) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let p = self.palette();
        let lang = self.prefs.language.clone();
        let mut chosen: Option<String> = None;
        let (width, pos) = match &popup {
            Popup::Settings => (220.0, pos2(screen.right() - 330.0, 43.0)),
            Popup::Help => (440.0, pos2(20.0, 130.0)),
            Popup::Positions => (260.0, pos2(53.0, 130.0)),
            Popup::Search | Popup::Critical => (360.0, pos2(screen.right() - 408.0, 83.0)),
            Popup::Reports => (300.0, pos2(screen.right() - 321.0, 129.0)),
            Popup::Relations(_, _) | Popup::Unresolved(_) => {
                (420.0, pos2(60.0, (screen.bottom() - 430.0).max(100.0)))
            }
            Popup::Warnings | Popup::Licenses => {
                (screen.width().min(650.0) - 40.0, pos2(40.0, 105.0))
            }
            Popup::Examples => (210.0, screen.center() + vec2(-105.0, 110.0)),
        };
        let shown=Area::new(Id::new("popover")).order(Order::Foreground).fixed_pos(pos).show(ctx,|ui|{
            Frame::new().fill(p.popover).stroke(Stroke::new(1.0,p.line)).corner_radius(8).inner_margin(12).show(ui,|ui|{
                ui.set_width(width);ui.set_max_height((screen.bottom()-pos.y-36.0).max(160.0));
                match &popup {
                    Popup::Settings=>{
                        ui.label(RichText::new(tr(&lang,"settings")).size(11.0).color(p.muted));
                        for theme in ["light","dark"]{let caption=format!("{} {}",if self.prefs.theme==theme{"•"}else{" "},tr(&lang,theme));if self.menu_item(ui,&format!("theme-{theme}"),&caption,true){self.prefs.theme=theme.into();self.save_preferences();}}
                        ui.separator();ui.label(RichText::new(tr(&lang,"language")).size(11.0).color(p.muted));
                        for(code,name)in[("en","English"),("zh","简体中文"),("ja","日本語")]{let caption=format!("{} {name}",if self.prefs.language==code{"•"}else{" "});if self.menu_item(ui,&format!("language-{code}"),&caption,true){self.prefs.language=code.into();self.save_preferences();}}
                        ui.separator();let motion=match lang.as_str(){"zh"=>"减少动态效果","ja"=>"動きを減らす",_=>"Reduce motion"};let response=ui.checkbox(&mut self.prefs.reduced_motion,motion);register(&mut self.controls,"reduced-motion",response.rect);if response.changed(){self.save_preferences();if self.prefs.reduced_motion{self.finish();}}
                        if self.menu_item(ui,"licenses","RepoTower 0.6 · MIT / Licenses",true){self.popup=Some(Popup::Licenses);self.opened_popup=true;}
                    }
                    Popup::Help=>{ui.label(RichText::new(tr(&lang,"graph.help")).strong());ui.add_space(5.0);for line in text::help(&lang){ui.horizontal_wrapped(|ui|{ui.label(RichText::new("·").color(p.mint));ui.label(line);});ui.add_space(4.0);}}
                    Popup::Positions=>{let selected=self.simulation.selected.clone();if self.menu_item(ui,"reset-selected-position",tr(&lang,"graph.resetSelected"),selected.as_ref().is_some_and(|id|self.offsets.contains_key(id))){self.offsets.remove(selected.as_ref().unwrap());self.popup=None;}
                        if self.menu_item(ui,"reset-all-positions",tr(&lang,"graph.resetAll"),!self.offsets.is_empty()){self.offsets.clear();self.popup=None;}}
                    Popup::Examples=>{for(name,path)in[("JS / TS · Demo","demo-project"),("Java","language-samples/java"),("Python","language-samples/python"),("C","language-samples/c"),("C++","language-samples/cpp"),("Go","language-samples/go"),("Rust","language-samples/rust"),("C#","language-samples/csharp")]{if self.menu_item(ui,&format!("example-{path}"),name,true){self.open_example(path,ctx);}}}
                    Popup::Search|Popup::Critical|Popup::Relations(_,_)=>{
                        let(ids,title)=match &popup {Popup::Relations(id,callers)=>{let module=self.analysis.as_ref().and_then(|a|a.modules.iter().find(|m|m.id==*id));(module.map(|m|if *callers{m.imported_by.clone()}else{m.imports.clone()}).unwrap_or_default(),tr(&lang,if *callers{"dependents"}else{"dependencies"}))},Popup::Critical=>(self.search_results(true),tr(&lang,"critical")),_=>(self.search_results(false),tr(&lang,"all"))};
                        ui.label(RichText::new(title).color(p.muted));let a=self.analysis.clone();if ids.is_empty(){ui.label(tr(&lang,"emptySearch"));}
                        ScrollArea::vertical().max_height(280.0).show(ui,|ui|{for id in ids {if let Some(m)=a.as_ref().and_then(|a|a.modules.iter().find(|m|m.id==id)){let caption=format!("{}  ·  {}",text::short(&m.relative_path,44),m.blast_radius);if self.menu_item(ui,&format!("result-{id}"),&caption,true){chosen=Some(id);}}}});
                    }
                    Popup::Reports=>{ui.label(RichText::new(tr(&lang,"report")).color(p.muted));let reports=self.simulation.reports.clone();ScrollArea::vertical().max_height(260.0).show(ui,|ui|{for report in reports{let id=report.removed_module_id;let caption=format!("{}  ·  {}",text::short(&id,34),report.total_affected);if self.menu_item(ui,&format!("report-{id}"),&caption,self.simulation.live.is_none()){self.simulation.review=Some(id);self.playhead=self.max_wave();self.playing=false;self.view=View::Impact;self.layout_dirty=true;self.popup=None;}}});}
                    Popup::Warnings=>{ui.label(RichText::new(tr(&lang,"warnings")).strong());let a=self.analysis.clone();ScrollArea::vertical().max_height(330.0).show(ui,|ui|{if let Some(a)=a{for warning in &a.warnings{ui.label(text::diagnostic(warning,&lang));ui.add_space(8.0);}}});}
                    Popup::Unresolved(id)=>{ui.label(RichText::new(tr(&lang,"unresolved")).strong());let a=self.analysis.clone();ScrollArea::vertical().max_height(280.0).show(ui,|ui|{if let Some(m)=a.as_ref().and_then(|a|a.modules.iter().find(|m|m.id==*id)){for unresolved in &m.unresolved_imports{ui.label(RichText::new(&unresolved.specifier).monospace().color(p.affected));ui.label(text::diagnostic(&unresolved.reason,&lang));ui.add_space(8.0);}}});}
                    Popup::Licenses=>{ui.label(RichText::new("RepoTower 0.6.0").strong());ScrollArea::vertical().max_height(350.0).show(ui,|ui|{ui.label(include_str!("../../../LICENSE"));ui.separator();ui.label("Noto Sans CJK · Copyright 2014–2021 Adobe (http://www.adobe.com/), with Reserved Font Name ‘Source’.");ui.label(include_str!("../../../assets/fonts/OFL.txt"));ui.separator();ui.label(include_str!("../../../THIRD_PARTY_LICENSES.txt"));});}
                }
            });
        });
        self.popup_rect = shown.response.rect;
        if let Some(id) = chosen {
            self.select(Some(id.clone()));
            self.popup = None;
            self.reveal_selected(&id);
            ctx.memory_mut(|m| m.surrender_focus(Id::new("search-field")));
        }
    }
}
