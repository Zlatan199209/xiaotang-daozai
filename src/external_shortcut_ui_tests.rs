//! 外部通知常驻入口：通知带目标窗口时，收起态右侧留下图标，点击请求外壳切到该窗口。
use super::*;

const SOURCE:&str="claude.test";

fn ready() -> H {
    let mut h=H::new();h.frame(vec![]);
    h.app.settings.read_notifications=true;h.app.settings.notifications=true;
    h.frame(vec![]);
    h
}
/// egui 在每帧结束时交换 this_pass / prev_pass，而 `read_response` 先查 this_pass：
/// 刚跑完一帧时读到的是上一帧的控件。所以状态变化后要连跑两帧再读，和健康角标的测试一个做法。
fn settle(h:&mut H){h.frame(vec![]);h.frame(vec![]);}
fn receive(h:&mut H,window:isize,name:&str) {
    h.app.receive_external_target("Windows PowerShell".into(),SOURCE.into(),window,name.into());
    settle(h);
}
fn entry(h:&H,window:isize) -> Rect {
    h.ctx.read_response(egui::Id::new(("card",true,format!("external-{window}")))).unwrap_or_else(||panic!("缺少窗口 {window} 的常驻入口")).rect
}
fn missing(h:&H,window:isize) -> bool {
    h.ctx.read_response(egui::Id::new(("card",true,format!("external-{window}")))).is_none()
}
fn focus_requests(h:&H) -> Vec<isize> {
    h.cmds.iter().filter_map(|cmd|match cmd{crate::app::Cmd::FocusExternal(target)=>Some(target.window),_=>None}).collect()
}

#[test]
fn focus_requests_follow_the_launch_attribute_format(){
    assert_eq!(crate::system::parse_focus("daozai-focus:4242:Claude Code"),Some((4242,"Claude Code".into())));
    assert_eq!(crate::system::parse_focus("daozai-focus: 7 : Terminal "),Some((7,"Terminal".into())));
    assert_eq!(crate::system::parse_focus("daozai-focus:4242:"),Some((4242,"外部应用".into())));
    assert_eq!(crate::system::parse_focus("daozai-focus:4242:Cla\tude Code").unwrap().1,"Claude Code");
    let (_,name)=crate::system::parse_focus(&format!("daozai-focus:4242:{}","名".repeat(40))).unwrap();
    assert_eq!(name.chars().count(),24);
    for invalid in ["","https://example.com","daozai-focus:0:Claude Code","daozai-focus:abc:Claude Code","daozai-focus:4242"]{
        assert_eq!(crate::system::parse_focus(invalid),None,"{invalid:?} 不应被当成请求");
    }
}

#[test]
fn external_focus_request_leaves_a_persistent_entry(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();
    receive(&mut h,4242,"Claude Code");
    assert_eq!(h.app.external_targets.len(),1);assert_eq!(h.app.unread_count(),1);
    let rect=entry(&h,4242);
    assert!(rect.width()>10.0 && rect.height()>10.0,"{rect:?}");
    h.t+=10.0;h.frame(vec![]);
    assert!(entry(&h,4242).width()>10.0,"横幅消失后入口仍要保留");
}

#[test]
fn external_entries_dedupe_by_window_and_line_up(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();
    receive(&mut h,4242,"Claude Code");receive(&mut h,4242,"Claude Code 2");receive(&mut h,5151,"Claude Code");
    assert_eq!(h.app.external_targets.len(),2);
    assert_eq!(h.app.external_targets[0].name,"Claude Code 2","同一窗口只留一条，后到的更新名称");
    let (a,b)=(entry(&h,4242),entry(&h,5151));
    assert!(a.right()<=b.left()+0.5,"两个入口不能重叠：{a:?} {b:?}");
}

#[test]
fn clicking_an_external_entry_asks_the_shell_to_focus_its_window(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();
    receive(&mut h,4242,"Claude Code");
    let p=entry(&h,4242).center();
    h.hover(p,0.6);h.click(p);
    assert_eq!(focus_requests(&h),[4242]);
    assert_eq!(h.app.external_targets.len(),1,"点击只发出命令，由外壳在切换成功后清除");
}

#[test]
fn external_entries_clear_after_viewing_or_when_the_window_is_gone(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();
    receive(&mut h,4242,"Claude Code");receive(&mut h,5151,"Claude Code");
    assert!(h.app.viewed_external(4242));assert_eq!(h.app.external_targets.len(),1);
    settle(&mut h);assert!(missing(&h,4242));assert!(!missing(&h,5151));
    h.app.retain_external_targets(|window|window!=5151);assert!(h.app.external_targets.is_empty());
    settle(&mut h);assert!(missing(&h,5151));assert_eq!(h.app.unread_count(),0);
    assert!(!h.app.viewed_external(4242),"没有该窗口的入口时返回 false");
}

#[test]
fn external_entries_stay_clickable_in_the_expanded_capsule(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();
    receive(&mut h,4242,"Claude Code");
    h.app.reveal_unread();
    for _ in 0..120{h.frame(vec![]);}
    assert!(h.app.messenger_appear()>0.9);
    let p=entry(&h,4242).center();
    h.hover(p,0.3);h.click(p);
    assert_eq!(focus_requests(&h),[4242]);
}

#[test]
fn external_entries_follow_the_notification_switches(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();
    receive(&mut h,4242,"Claude Code");
    h.app.settings.qq_only=true;h.frame(vec![]);assert!(h.app.external_targets.is_empty(),"只连接 QQ 时不保留");
    h.app.settings.qq_only=false;receive(&mut h,4242,"Claude Code");assert_eq!(h.app.external_targets.len(),1);
    h.app.notification_apps.items[0].2=false;h.frame(vec![]);assert!(h.app.external_targets.is_empty(),"来源被关闭后不保留");
    h.app.notification_apps.items.clear();receive(&mut h,4242,"Claude Code");assert_eq!(h.app.external_targets.len(),1,"新来源默认放行");
    h.app.settings.read_notifications=false;h.frame(vec![]);assert!(h.app.external_targets.is_empty(),"断开系统通知后不保留");
    h.app.settings.read_notifications=true;receive(&mut h,0,"Claude Code");assert!(h.app.external_targets.is_empty(),"句柄为 0 的请求忽略");
    receive(&mut h,4242,"Claude Code");h.app.settings.notifications=false;h.frame(vec![]);
    assert!(h.app.external_targets.is_empty(),"总开关关闭后不保留");
}

#[test]
fn external_entries_leave_messenger_unread_untouched(){
    let _g=ENV_LOCK.lock().unwrap_or_else(|e|e.into_inner());let mut h=ready();h.app.settings.wechat_notifications=true;
    h.app.receive_wechat_reminder();h.app.alerts.clear();h.app.notice=None;
    receive(&mut h,4242,"Claude Code");
    assert_eq!(h.app.pending_messengers.len(),1);assert_eq!(h.app.unread_count(),2);
    let wechat=h.ctx.read_response(egui::Id::new(("card",true,"微信"))).unwrap_or_else(||panic!("缺少微信入口")).rect;
    let external=entry(&h,4242);
    assert!(wechat.right()<=external.left()+0.5,"微信入口与外部入口不能重叠：{wechat:?} {external:?}");
    h.hover(wechat.center(),0.6);h.click(wechat.center());
    assert!(h.cmds.iter().any(|cmd|matches!(cmd,crate::app::Cmd::ViewMessenger(unread) if unread.source==crate::app::Messenger::Wechat)),"微信入口仍要发出查看命令：{:?}",h.cmds);
    assert!(focus_requests(&h).is_empty());
    h.cmds.clear();h.cursor=None;h.frame(vec![]);
    let external=entry(&h,4242);h.hover(external.center(),0.3);h.click(external.center());
    assert_eq!(focus_requests(&h),[4242]);
}

#[test]
fn focus_requests_can_also_arrive_at_the_end_of_the_body(){
    let (body,focus)=crate::system::body_focus("Claude Code · 执行完成 · 16:00:00 · daozai-focus:4242:Claude Code");
    assert_eq!(focus,Some((4242,"Claude Code".into())));assert_eq!(body,"Claude Code · 执行完成 · 16:00:00");
    let (body,focus)=crate::system::body_focus("没有请求的正文");
    assert_eq!((body.as_str(),focus),("没有请求的正文",None));
}
