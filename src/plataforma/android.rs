//! Acesso às APIs do Android via JNI:
//! - UsageStatsManager.queryEvents  -> quando cada app foi aberto/fechado
//! - PackageManager                 -> nome dos apps e se são jogos
//! - AppOpsManager                  -> se a permissão "Acesso ao uso" foi dada

use std::path::PathBuf;

use jni::errors::Result as JniResult;
use jni::objects::{JByteArray, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM};
use winit::platform::android::activity::AndroidApp;

use super::{ler_conta, ler_fonte, Conta, EstadoFonte, Fonte, InfoApp, PedidoImagem, Plataforma};
use crate::sessoes::{self, Evento};

const CATEGORIA_JOGO: i32 = 0; // ApplicationInfo.CATEGORY_GAME
const FLAG_JOGO: i32 = 0x0200_0000; // ApplicationInfo.FLAG_IS_GAME (antigo, mas muitos jogos ainda usam)
const MODO_PERMITIDO: i32 = 0; // AppOpsManager.MODE_ALLOWED

pub struct PlataformaAndroid {
    vm: JavaVM,
    /// Referência global da Activity (guardada como número para poder ir entre threads).
    atividade: usize,
    pasta: PathBuf,
    pacote: String,
}

impl PlataformaAndroid {
    pub fn nova(app: &AndroidApp) -> Self {
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut jni::sys::JavaVM) }
            .expect("não consegui acessar a JavaVM");
        let pasta = app
            .internal_data_path()
            .unwrap_or_else(|| PathBuf::from("/data/local/tmp/abysshub"));
        let mut plat = PlataformaAndroid {
            vm,
            atividade: app.activity_as_ptr() as usize,
            pasta,
            pacote: String::new(),
        };
        plat.pacote = plat
            .com_env(|env, act| {
                let p = env
                    .call_method(act, "getPackageName", "()Ljava/lang/String;", &[])?
                    .l()?;
                texto(env, p)
            })
            .unwrap_or_else(|_| "com.abysshub.app".to_string());
        plat
    }

    /// Chama um método "static void nome()" do MainActivity.java.
    fn estatico_void(&self, nome: &str) {
        let _ = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            env.call_static_method(&classe, nome, "()V", &[])?;
            Ok(())
        });
    }

    /// Chama um método "static String nome(String...)" do MainActivity.java.
    fn estatico_texto(&self, nome: &str, args: &[&str]) -> Result<String, String> {
        self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let mut objs = Vec::with_capacity(args.len());
            for a in args {
                objs.push(JObject::from(env.new_string(a)?));
            }
            let valores: Vec<JValue> = objs.iter().map(JValue::Object).collect();
            let assinatura = format!("({})Ljava/lang/String;", "Ljava/lang/String;".repeat(args.len()));
            let r = env.call_static_method(&classe, nome, assinatura, &valores)?.l()?;
            texto(env, r)
        })
    }

    /// Roda `f` com um JNIEnv da thread atual e a Activity.
    fn com_env<T>(&self, f: impl FnOnce(&mut JNIEnv, &JObject) -> JniResult<T>) -> Result<T, String> {
        let mut env = self
            .vm
            .attach_current_thread_permanently()
            .map_err(|e| format!("JNI: {e}"))?;
        // A Activity do NativeActivity é uma referência global, válida em qualquer thread.
        let act = unsafe { JObject::from_raw(self.atividade as jni::sys::jobject) };
        let r = env.with_local_frame(64, |env| f(env, &act));
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_describe();
            let _ = env.exception_clear();
        }
        r.map_err(|e| format!("Erro do Android: {e}"))
    }
}

/// Converte uma String Java em String Rust e libera a referência local.
fn texto(env: &mut JNIEnv, obj: JObject) -> JniResult<String> {
    if obj.is_null() {
        return Ok(String::new());
    }
    let js = JString::from(obj);
    let s: String = env.get_string(&js)?.into();
    env.delete_local_ref(js)?;
    Ok(s)
}

fn servico<'a>(env: &mut JNIEnv<'a>, act: &JObject, nome: &str) -> JniResult<JObject<'a>> {
    let n = env.new_string(nome)?;
    env.call_method(
        act,
        "getSystemService",
        "(Ljava/lang/String;)Ljava/lang/Object;",
        &[JValue::Object(&n)],
    )?
    .l()
}

impl Plataforma for PlataformaAndroid {
    fn tem_permissao(&self) -> bool {
        self.com_env(|env, act| {
            let ops = servico(env, act, "appops")?;
            let uid = env.call_static_method("android/os/Process", "myUid", "()I", &[])?.i()?;
            let op = env.new_string("android:get_usage_stats")?;
            let pkg = env.new_string(&self.pacote)?;
            let modo = env
                .call_method(
                    &ops,
                    "checkOpNoThrow",
                    "(Ljava/lang/String;ILjava/lang/String;)I",
                    &[JValue::Object(&op), JValue::Int(uid), JValue::Object(&pkg)],
                )?
                .i()?;
            Ok(modo == MODO_PERMITIDO)
        })
        .unwrap_or(false)
    }

    fn abrir_permissao(&self) {
        let r = self.com_env(|env, act| {
            let acao = env.new_string("android.settings.USAGE_ACCESS_SETTINGS")?;
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&acao)],
            )?;
            env.call_method(
                act,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        });
        if let Err(e) = r {
            log::error!("não abriu as configurações: {e}");
        }
    }

    fn listar_apps(&self) -> Result<Vec<InfoApp>, String> {
        self.com_env(|env, act| {
            let pm = env
                .call_method(act, "getPackageManager", "()Landroid/content/pm/PackageManager;", &[])?
                .l()?;
            let lista = env
                .call_method(
                    &pm,
                    "getInstalledApplications",
                    "(I)Ljava/util/List;",
                    &[JValue::Int(0)],
                )?
                .l()?;
            let n = env.call_method(&lista, "size", "()I", &[])?.i()?;

            let mut apps = Vec::with_capacity(n.max(0) as usize);
            for i in 0..n {
                // Um "frame" por app: as referências locais são liberadas no fim de cada volta.
                let r = env.with_local_frame(16, |env| -> JniResult<InfoApp> {
                    let ai = env
                        .call_method(&lista, "get", "(I)Ljava/lang/Object;", &[JValue::Int(i)])?
                        .l()?;
                    let pkg_obj = env.get_field(&ai, "packageName", "Ljava/lang/String;")?.l()?;
                    let pkg_js = JString::from(pkg_obj);
                    let pacote: String = env.get_string(&pkg_js)?.into();
                    let flags = env.get_field(&ai, "flags", "I")?.i()?;
                    let categoria = env.get_field(&ai, "category", "I")?.i()?;

                    let rotulo = env
                        .call_method(
                            &pm,
                            "getApplicationLabel",
                            "(Landroid/content/pm/ApplicationInfo;)Ljava/lang/CharSequence;",
                            &[JValue::Object(&ai)],
                        )?
                        .l()?;
                    let rotulo_s = env.call_method(&rotulo, "toString", "()Ljava/lang/String;", &[])?.l()?;
                    let nome = texto(env, rotulo_s)?;

                    let abrir = env
                        .call_method(
                            &pm,
                            "getLaunchIntentForPackage",
                            "(Ljava/lang/String;)Landroid/content/Intent;",
                            &[JValue::Object(&pkg_js)],
                        )?
                        .l()?;

                    Ok(InfoApp {
                        nome: if nome.is_empty() { pacote.clone() } else { nome },
                        pacote,
                        jogo: categoria == CATEGORIA_JOGO || flags & FLAG_JOGO != 0,
                        abrivel: !abrir.is_null(),
                    })
                });
                match r {
                    Ok(app) => apps.push(app),
                    Err(e) => {
                        // Um app com problema não pode derrubar a lista inteira.
                        let _ = env.exception_clear();
                        log::warn!("pulei um app: {e}");
                    }
                }
            }
            Ok(apps)
        })
    }

    fn eventos(&self, desde: i64, ate: i64) -> Result<Vec<Evento>, String> {
        self.com_env(|env, act| {
            let usm = servico(env, act, "usagestats")?;
            let evs = env
                .call_method(
                    &usm,
                    "queryEvents",
                    "(JJ)Landroid/app/usage/UsageEvents;",
                    &[JValue::Long(desde), JValue::Long(ate)],
                )?
                .l()?;
            if evs.is_null() {
                return Ok(Vec::new());
            }
            let ev = env.new_object("android/app/usage/UsageEvents$Event", "()V", &[])?;

            let mut saida = Vec::new();
            while env.call_method(&evs, "hasNextEvent", "()Z", &[])?.z()? {
                env.call_method(
                    &evs,
                    "getNextEvent",
                    "(Landroid/app/usage/UsageEvents$Event;)Z",
                    &[JValue::Object(&ev)],
                )?;
                let tipo = env.call_method(&ev, "getEventType", "()I", &[])?.i()?;
                if !sessoes::evento_relevante(tipo) {
                    continue;
                }
                let ts = env.call_method(&ev, "getTimeStamp", "()J", &[])?.j()?;
                let p = env
                    .call_method(&ev, "getPackageName", "()Ljava/lang/String;", &[])?
                    .l()?;
                let pacote = texto(env, p)?;
                let c = env.call_method(&ev, "getClassName", "()Ljava/lang/String;", &[])?.l()?;
                let classe = texto(env, c)?;
                saida.push(Evento {
                    pacote,
                    classe,
                    tipo,
                    ts,
                });
            }
            Ok(saida)
        })
    }

    fn pasta_dados(&self) -> PathBuf {
        self.pasta.clone()
    }

    fn pacote_proprio(&self) -> String {
        self.pacote.clone()
    }

    fn tema_escuro_sistema(&self) -> Option<bool> {
        self.com_env(|env, act| {
            let res = env
                .call_method(act, "getResources", "()Landroid/content/res/Resources;", &[])?
                .l()?;
            let conf = env
                .call_method(&res, "getConfiguration", "()Landroid/content/res/Configuration;", &[])?
                .l()?;
            let modo = env.get_field(&conf, "uiMode", "I")?.i()?;
            // Configuration.UI_MODE_NIGHT_MASK = 0x30, UI_MODE_NIGHT_YES = 0x20
            Ok((modo & 0x30) == 0x20)
        })
        .ok()
    }

    fn margens_sistema(&self) -> Option<(f32, f32)> {
        self.com_env(|env, act| {
            let janela = env.call_method(act, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
            let decor = env
                .call_method(&janela, "getDecorView", "()Landroid/view/View;", &[])?
                .l()?;
            let ins = env
                .call_method(&decor, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?
                .l()?;
            if ins.is_null() {
                return Ok(None); // a janela ainda não está pronta; tenta de novo depois
            }
            let topo = env.call_method(&ins, "getSystemWindowInsetTop", "()I", &[])?.i()?;
            let baixo = env.call_method(&ins, "getSystemWindowInsetBottom", "()I", &[])?.i()?;
            Ok(Some((topo as f32, baixo as f32)))
        })
        .ok()
        .flatten()
    }

    // ---------- Conta e Leaderboard: métodos "static" do MainActivity.java ----------

    fn conta(&self) -> Conta {
        self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let r = env
                .call_static_method(&classe, "estadoConta", "()Ljava/lang/String;", &[])?
                .l()?;
            texto(env, r)
        })
        .map(|j| ler_conta(&j))
        .unwrap_or_default()
    }

    fn entrar(&self) {
        let _ = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            env.call_static_method(&classe, "entrar", "()V", &[])?;
            Ok(())
        });
    }

    fn sair(&self) {
        let _ = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            env.call_static_method(&classe, "sair", "()V", &[])?;
            Ok(())
        });
    }

    fn publicar(&self, json: &str) -> Result<(), String> {
        let r = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let s = env.new_string(json)?;
            let r = env
                .call_static_method(
                    &classe,
                    "publicar",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    &[JValue::Object(&s)],
                )?
                .l()?;
            texto(env, r)
        })?;
        if r.is_empty() {
            Ok(())
        } else {
            Err(r)
        }
    }

    fn buscar_ranking(
        &self,
        modo: &str,
        periodo: &str,
        plataforma: &str,
        chave: &str,
        dia: &str,
    ) -> Result<String, String> {
        let r = self.estatico_texto("buscar", &[modo, periodo, plataforma, chave, dia])?;
        match r.strip_prefix("ERRO:") {
            Some(e) => Err(e.to_string()),
            None => Ok(r),
        }
    }

    // MainActivity.java: steamEstado/psnEstado, steamConectar/psnConectar...
    fn fonte_estado(&self, fonte: Fonte) -> EstadoFonte {
        self.estatico_texto(&format!("{}Estado", fonte.chave()), &[])
            .map(|j| ler_fonte(&j))
            .unwrap_or_default()
    }

    fn fonte_conectar(&self, fonte: Fonte) {
        self.estatico_void(&format!("{}Conectar", fonte.chave()));
    }

    fn fonte_desconectar(&self, fonte: Fonte) {
        self.estatico_void(&format!("{}Desconectar", fonte.chave()));
    }

    fn fonte_jogos(&self, fonte: Fonte) -> Result<Option<String>, String> {
        let r = self.estatico_texto(&format!("{}Jogos", fonte.chave()), &[])?;
        if r == "DESLIGADO" {
            Ok(None)
        } else if let Some(e) = r.strip_prefix("ERRO:") {
            Err(e.to_string())
        } else {
            Ok(Some(r))
        }
    }

    fn usar_moldura(&self, id: &str) -> Result<(), String> {
        let r = self.estatico_texto("usarMoldura", &[id])?;
        if r.is_empty() {
            Ok(())
        } else {
            Err(r)
        }
    }

    fn publicar_historico(&self, json: &str) -> Result<(), String> {
        let r = self.estatico_texto("publicarHistorico", &[json])?;
        if r.is_empty() {
            Ok(())
        } else {
            Err(r)
        }
    }

    fn historicos(&self) -> Result<String, String> {
        let r = self.estatico_texto("historicos", &[])?;
        match r.strip_prefix("ERRO:") {
            Some(e) => Err(e.to_string()),
            None => Ok(r),
        }
    }

    fn buscar_eventos(&self) -> Result<String, String> {
        let r = self.estatico_texto("listaEventos", &[])?;
        match r.strip_prefix("ERRO:") {
            Some(e) => Err(e.to_string()),
            None => Ok(r),
        }
    }

    fn eventos_versao(&self) -> i64 {
        self.estatico_texto("eventosVersao", &[])
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(0)
    }

    fn evento_editar(&self, evento: &str, jogos: &str) {
        let _ = self.estatico_texto("eventoEditar", &[evento, jogos]);
    }

    fn evento_premios(&self, id: &str, premios: &str) -> Result<(), String> {
        let r = self.estatico_texto("eventoPremios", &[id, premios])?;
        if r.is_empty() {
            Ok(())
        } else {
            Err(r)
        }
    }

    fn evento_apagar(&self, id: &str) {
        let _ = self.estatico_texto("eventoApagar", &[id]);
    }

    fn escolher_data(&self, atual: &str) {
        let _ = self.estatico_texto("escolherData", &[atual]);
    }

    fn data_escolhida(&self) -> Option<String> {
        self.estatico_texto("dataEscolhida", &[])
            .ok()
            .filter(|t| !t.is_empty())
    }

    fn apagar_meus_dados(&self) -> Result<(), String> {
        let r = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let r = env
                .call_static_method(&classe, "apagarMeusDados", "()Ljava/lang/String;", &[])?
                .l()?;
            texto(env, r)
        })?;
        if r.is_empty() {
            Ok(())
        } else {
            Err(r)
        }
    }

    fn imagem(&self, pedido: &PedidoImagem, tam: u32) -> Option<Vec<u8>> {
        let (metodo, arg) = match pedido {
            PedidoImagem::Icone(p) => ("iconeApp", p.as_str()),
            PedidoImagem::Foto(u) => ("baixarImagem", u.as_str()),
        };
        let bytes = self
            .com_env(|env, act| {
                let classe = env.get_object_class(act)?;
                let s = env.new_string(arg)?;
                let r = env
                    .call_static_method(
                        &classe,
                        metodo,
                        "(Ljava/lang/String;I)[B",
                        &[JValue::Object(&s), JValue::Int(tam as i32)],
                    )?
                    .l()?;
                if r.is_null() {
                    return Ok(None);
                }
                let arr = JByteArray::from(r);
                Ok(Some(env.convert_byte_array(&arr)?))
            })
            .ok()
            .flatten()?;
        if bytes.len() == (tam * tam * 4) as usize {
            Some(bytes)
        } else {
            None
        }
    }

    fn abrir_link(&self, url: &str) {
        let _ = self.com_env(|env, act| {
            let s = env.new_string(url)?;
            let uri = env
                .call_static_method(
                    "android/net/Uri",
                    "parse",
                    "(Ljava/lang/String;)Landroid/net/Uri;",
                    &[JValue::Object(&s)],
                )?
                .l()?;
            let acao = env.new_string("android.intent.action.VIEW")?;
            let intent = env.new_object(
                "android/content/Intent",
                "(Ljava/lang/String;Landroid/net/Uri;)V",
                &[JValue::Object(&acao), JValue::Object(&uri)],
            )?;
            env.call_method(
                act,
                "startActivity",
                "(Landroid/content/Intent;)V",
                &[JValue::Object(&intent)],
            )?;
            Ok(())
        });
    }

    fn entrar_email(&self) {
        self.estatico_void("entrarComEmail");
    }

    fn editar_nome(&self) {
        self.estatico_void("editarNome");
    }

    fn editar_foto(&self) {
        self.estatico_void("editarFoto");
    }

    fn ligar_google(&self) {
        self.estatico_void("ligarGoogle");
    }

    fn ligar_email(&self) {
        self.estatico_void("ligarEmail");
    }

    fn verificar_atualizacao(&self, repo: &str) -> Result<Option<String>, String> {
        let r = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let s = env.new_string(repo)?;
            let r = env
                .call_static_method(
                    &classe,
                    "verificarAtualizacao",
                    "(Ljava/lang/String;)Ljava/lang/String;",
                    &[JValue::Object(&s)],
                )?
                .l()?;
            texto(env, r)
        })?;
        if r == "NENHUMA" {
            Ok(None)
        } else if let Some(e) = r.strip_prefix("ERRO:") {
            Err(e.to_string())
        } else {
            Ok(Some(r))
        }
    }

    fn baixar_atualizacao(&self, url: &str) {
        let _ = self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let s = env.new_string(url)?;
            env.call_static_method(
                &classe,
                "baixarAtualizacao",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&s)],
            )?;
            Ok(())
        });
    }

    fn instalar_atualizacao(&self) {
        self.estatico_void("instalarAtualizacao");
    }

    fn estado_atualizacao(&self) -> String {
        self.com_env(|env, act| {
            let classe = env.get_object_class(act)?;
            let r = env
                .call_static_method(&classe, "estadoAtualizacao", "()Ljava/lang/String;", &[])?
                .l()?;
            texto(env, r)
        })
        .unwrap_or_default()
    }
}
