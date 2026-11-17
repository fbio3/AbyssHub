package com.abysshub.app;

import android.app.AlertDialog;
import android.app.NativeActivity;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.Settings;
import android.text.InputType;
import android.view.Gravity;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Rect;
import android.graphics.drawable.Drawable;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.os.Handler;
import android.os.Looper;
import android.util.Base64;
import android.util.Log;

import androidx.credentials.ClearCredentialStateRequest;
import androidx.credentials.Credential;
import androidx.credentials.CredentialManager;
import androidx.credentials.CredentialManagerCallback;
import androidx.credentials.CustomCredential;
import androidx.credentials.GetCredentialRequest;
import androidx.credentials.GetCredentialResponse;
import androidx.credentials.exceptions.ClearCredentialException;
import androidx.credentials.exceptions.GetCredentialCancellationException;
import androidx.credentials.exceptions.GetCredentialException;
import androidx.credentials.exceptions.NoCredentialException;

import com.google.android.gms.tasks.Tasks;
import com.google.android.libraries.identity.googleid.GetGoogleIdOption;
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential;
import com.google.firebase.FirebaseApp;
import com.google.firebase.auth.AuthCredential;
import androidx.core.content.FileProvider;

import com.google.firebase.auth.FirebaseAuth;
import com.google.firebase.auth.FirebaseAuthInvalidCredentialsException;
import com.google.firebase.auth.FirebaseAuthInvalidUserException;
import com.google.firebase.auth.FirebaseAuthUserCollisionException;
import com.google.firebase.auth.FirebaseAuthWeakPasswordException;
import com.google.firebase.auth.UserProfileChangeRequest;
import com.google.firebase.auth.FirebaseUser;
import com.google.firebase.auth.GoogleAuthProvider;
import com.google.firebase.firestore.DocumentReference;
import com.google.firebase.firestore.DocumentSnapshot;
import com.google.firebase.firestore.FirebaseFirestore;
import com.google.firebase.firestore.Query;
import com.google.firebase.firestore.QuerySnapshot;
import com.google.firebase.firestore.WriteBatch;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.ByteBuffer;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.Executor;
import java.util.concurrent.TimeUnit;

/**
 * A tela inteira é desenhada pelo Rust (NativeActivity).
 * Esta classe só faz o que o Android exige em Java: login com Google, Firebase e imagens.
 * O Rust chama os métodos "static" abaixo pelo JNI.
 */
public class MainActivity extends NativeActivity {
    private static final String TAG = "AbyssHub";
    private static final String PESSOAS = "abysshub_pessoas";
    private static final String PARES = "abysshub_pares";
    private static final String ICONES = "abysshub_icones";
    private static final long SEMANA_MS = 7L * 24 * 60 * 60 * 1000;

    static volatile MainActivity ativa;
    /** "saiu", "entrando" ou "erro" (logado vem do próprio FirebaseAuth) */
    private static volatile String estado = "saiu";
    private static volatile String mensagem = "";

    private static final Executor PRINCIPAL = r -> new Handler(Looper.getMainLooper()).post(r);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        ativa = this;
        super.onCreate(savedInstanceState);
    }

    @Override
    protected void onResume() {
        super.onResume();
        // voltou da tela "Instalar apps desconhecidos": continua a instalação sozinho
        if ("permissao".equals(updEstado) && podeInstalar(this)) instalarAtualizacao();
    }

    @Override
    protected void onDestroy() {
        if (ativa == this) ativa = null;
        super.onDestroy();
    }

    // =================================================================
    // Conta
    // =================================================================

    private static boolean firebasePronto(Context c) {
        try {
            return c != null && !FirebaseApp.getApps(c).isEmpty() && idWeb(c) != 0;
        } catch (Throwable t) {
            return false;
        }
    }

    private static int idWeb(Context c) {
        return c.getResources().getIdentifier("default_web_client_id", "string", c.getPackageName());
    }

    /** JSON: {"estado": "indisponivel|saiu|entrando|logado|erro", "msg", "uid", "nome", "foto"} */
    public static String estadoConta() {
        JSONObject o = new JSONObject();
        try {
            MainActivity a = ativa;
            if (!firebasePronto(a)) {
                o.put("estado", "indisponivel");
                o.put("msg", "O login não está disponível nesta versão do app.");
                return o.toString();
            }
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u != null) {
                o.put("estado", "logado");
                o.put("uid", u.getUid());
                o.put("nome", nomeDe(u));
                o.put("foto", fotoDe(u));
            } else {
                o.put("estado", estado);
                o.put("msg", mensagem);
            }
        } catch (Throwable t) {
            Log.e(TAG, "estadoConta", t);
        }
        return o.toString();
    }

    private static String nomeDe(FirebaseUser u) {
        String n = u.getDisplayName();
        if (n == null || n.trim().isEmpty()) n = "Jogador";
        return n.trim();
    }

    private static String fotoDe(FirebaseUser u) {
        if (u.getPhotoUrl() == null) return "";
        // foto maior que a padrão (96 px)
        return u.getPhotoUrl().toString().replace("s96-c", "s256-c");
    }

    /** Abre a janelinha de contas do Google. O resultado aparece em estadoConta(). */
    public static void entrar() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            if (!firebasePronto(a)) {
                estado = "erro";
                mensagem = "O login não está disponível nesta versão do app.";
                return;
            }
            estado = "entrando";
            mensagem = "";
            try {
                GetGoogleIdOption opcao = new GetGoogleIdOption.Builder()
                        .setFilterByAuthorizedAccounts(false)
                        .setServerClientId(a.getString(idWeb(a)))
                        .setAutoSelectEnabled(false)
                        .build();
                GetCredentialRequest pedido = new GetCredentialRequest.Builder()
                        .addCredentialOption(opcao)
                        .build();
                CredentialManager.create(a).getCredentialAsync(a, pedido, new CancellationSignal(), PRINCIPAL,
                        new CredentialManagerCallback<GetCredentialResponse, GetCredentialException>() {
                            @Override
                            public void onResult(GetCredentialResponse r) {
                                usarCredencial(r.getCredential());
                            }

                            @Override
                            public void onError(GetCredentialException e) {
                                if (e instanceof GetCredentialCancellationException) {
                                    estado = "saiu";
                                    mensagem = "";
                                } else if (e instanceof NoCredentialException) {
                                    falhou("Nenhuma conta Google neste celular.");
                                } else {
                                    falhou("Não foi possível entrar com o Google: " + e.getMessage());
                                }
                            }
                        });
            } catch (Throwable t) {
                falhou("Não foi possível entrar com o Google: " + t.getMessage());
            }
        });
    }

    private static void usarCredencial(Credential c) {
        try {
            if (c instanceof CustomCredential
                    && GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL.equals(c.getType())) {
                GoogleIdTokenCredential g = GoogleIdTokenCredential.createFrom(c.getData());
                AuthCredential cred = GoogleAuthProvider.getCredential(g.getIdToken(), null);
                FirebaseAuth.getInstance().signInWithCredential(cred).addOnCompleteListener(t -> {
                    if (t.isSuccessful()) {
                        estado = "saiu"; // a partir daqui vale o FirebaseAuth (logado)
                        mensagem = "";
                    } else {
                        falhou("O Firebase recusou o login: "
                                + (t.getException() != null ? t.getException().getMessage() : ""));
                    }
                });
            } else {
                falhou("Tipo de conta não suportado.");
            }
        } catch (Throwable t) {
            falhou("Erro no login: " + t.getMessage());
        }
    }

    private static void falhou(String msg) {
        Log.w(TAG, msg);
        estado = "erro";
        mensagem = msg;
    }

    public static void sair() {
        MainActivity a = ativa;
        try {
            FirebaseAuth.getInstance().signOut();
        } catch (Throwable ignored) {
        }
        estado = "saiu";
        mensagem = "";
        if (a == null) return;
        a.runOnUiThread(() -> {
            try {
                CredentialManager.create(a).clearCredentialStateAsync(new ClearCredentialStateRequest(),
                        new CancellationSignal(), PRINCIPAL,
                        new CredentialManagerCallback<Void, ClearCredentialException>() {
                            @Override public void onResult(Void v) { }
                            @Override public void onError(ClearCredentialException e) { }
                        });
            } catch (Throwable ignored) {
            }
        });
    }

    // =================================================================
    // Login com e-mail e senha (janelinha nativa, para o teclado funcionar direito)
    // =================================================================

    public static void entrarComEmail() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> a.dialogoEmail(false));
    }

    private int dp(int v) {
        return (int) (v * getResources().getDisplayMetrics().density);
    }

    private boolean escuro() {
        return (getResources().getConfiguration().uiMode & 0x30) == 0x20;
    }

    private EditText campo(String dica, int tipo) {
        EditText e = new EditText(this);
        e.setHint(dica);
        e.setInputType(tipo);
        e.setSingleLine(true);
        return e;
    }

    private void dialogoEmail(boolean criar) {
        if (!firebasePronto(this)) {
            falhou("O login não está disponível nesta versão do app.");
            return;
        }
        LinearLayout l = new LinearLayout(this);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setPadding(dp(22), dp(8), dp(22), 0);
        final EditText nome = criar
                ? campo("Seu nome (aparece no Leaderboard)", InputType.TYPE_CLASS_TEXT
                        | InputType.TYPE_TEXT_VARIATION_PERSON_NAME | InputType.TYPE_TEXT_FLAG_CAP_WORDS)
                : null;
        if (nome != null) l.addView(nome);
        final EditText email = campo("E-mail", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS);
        final EditText senha = campo(criar ? "Senha (mínimo 6 caracteres)" : "Senha",
                InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
        l.addView(email);
        l.addView(senha);
        if (!criar) {
            TextView esqueci = new TextView(this);
            esqueci.setText("Esqueci minha senha");
            esqueci.setPadding(0, dp(10), 0, dp(4));
            esqueci.setGravity(Gravity.END);
            esqueci.setTextColor(0xFF6F8FE0);
            esqueci.setOnClickListener(v -> {
                String e = email.getText().toString().trim();
                if (e.isEmpty()) {
                    email.setError("Digite seu e-mail primeiro");
                    return;
                }
                FirebaseAuth.getInstance().sendPasswordResetEmail(e).addOnCompleteListener(t ->
                        Toast.makeText(this, t.isSuccessful()
                                ? "Enviamos um link para trocar a senha no seu e-mail."
                                : "Não foi possível enviar: " + traduzir(t.getException()), Toast.LENGTH_LONG).show());
            });
            l.addView(esqueci);
        }

        int tema = escuro() ? android.R.style.Theme_DeviceDefault_Dialog_Alert
                : android.R.style.Theme_DeviceDefault_Light_Dialog_Alert;
        AlertDialog d = new AlertDialog.Builder(this, tema)
                .setTitle(criar ? "Criar conta" : "Entrar com e-mail")
                .setView(l)
                .setPositiveButton(criar ? "Criar conta" : "Entrar", null)
                .setNeutralButton(criar ? "Já tenho conta" : "Criar conta", null)
                .setNegativeButton("Cancelar", null)
                .create();
        d.setOnCancelListener(x -> voltarSeEntrando());
        d.setOnShowListener(x -> {
            d.getButton(DialogInterface.BUTTON_NEGATIVE).setOnClickListener(v -> {
                voltarSeEntrando();
                d.dismiss();
            });
            d.getButton(DialogInterface.BUTTON_NEUTRAL).setOnClickListener(v -> {
                d.dismiss();
                dialogoEmail(!criar);
            });
            d.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener(v -> {
                String e = email.getText().toString().trim();
                String s = senha.getText().toString();
                String n = nome != null ? nome.getText().toString().trim() : "";
                if (nome != null && n.isEmpty()) { nome.setError("Digite seu nome"); return; }
                if (e.isEmpty()) { email.setError("Digite seu e-mail"); return; }
                if (s.length() < 6) { senha.setError("A senha precisa de pelo menos 6 caracteres"); return; }
                estado = "entrando";
                mensagem = "";
                v.setEnabled(false);
                FirebaseAuth auth = FirebaseAuth.getInstance();
                (criar ? auth.createUserWithEmailAndPassword(e, s) : auth.signInWithEmailAndPassword(e, s))
                        .addOnCompleteListener(t -> {
                            if (!t.isSuccessful()) {
                                v.setEnabled(true);
                                String msg = traduzir(t.getException());
                                Toast.makeText(this, msg, Toast.LENGTH_LONG).show();
                                estado = "saiu";
                                return;
                            }
                            FirebaseUser u = auth.getCurrentUser();
                            if (criar && u != null) {
                                u.updateProfile(new UserProfileChangeRequest.Builder().setDisplayName(n).build())
                                        .addOnCompleteListener(x2 -> u.reload());
                            }
                            estado = "saiu"; // a partir daqui vale o FirebaseAuth (logado)
                            d.dismiss();
                        });
            });
        });
        estado = "entrando";
        d.show();
    }

    private static void voltarSeEntrando() {
        if ("entrando".equals(estado)) {
            estado = "saiu";
            mensagem = "";
        }
    }

    private static String traduzir(Exception e) {
        if (e instanceof FirebaseAuthWeakPasswordException) return "Senha fraca: use pelo menos 6 caracteres.";
        if (e instanceof FirebaseAuthUserCollisionException) return "Esse e-mail já tem uma conta. Toque em \"Já tenho conta\".";
        if (e instanceof FirebaseAuthInvalidUserException) return "Não existe conta com esse e-mail.";
        if (e instanceof FirebaseAuthInvalidCredentialsException) return "E-mail ou senha incorretos.";
        return e == null ? "Erro desconhecido." : mensagemDe(e);
    }

    // =================================================================
    // Atualizações (GitHub Releases), igual ao AbyssSub
    // =================================================================

    /** "parado" | "baixando" | "baixado" | "permissao" | "instalando" | "erro" */
    private static volatile String updEstado = "parado";
    private static volatile int updProgresso = 0;
    private static volatile String updMsg = "";

    /** Devolve o JSON da última versão, "NENHUMA" (ainda não há versões) ou "ERRO:mensagem". */
    public static String verificarAtualizacao(String repo) {
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL("https://api.github.com/repos/" + repo + "/releases/latest").openConnection();
            c.setRequestProperty("Accept", "application/vnd.github+json");
            c.setRequestProperty("User-Agent", "AbyssHub");
            c.setConnectTimeout(10000);
            c.setReadTimeout(10000);
            int codigo = c.getResponseCode();
            if (codigo == 404) return "NENHUMA";
            if (codigo != 200) return "ERRO:HTTP " + codigo;
            ByteArrayOutputStream b = new ByteArrayOutputStream();
            try (InputStream in = c.getInputStream()) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) > 0) b.write(buf, 0, n);
            }
            JSONObject r = new JSONObject(b.toString("UTF-8"));
            JSONObject o = new JSONObject();
            o.put("tag", r.optString("tag_name"));
            o.put("notas", r.optString("body"));
            o.put("pagina", r.optString("html_url"));
            JSONArray assets = r.optJSONArray("assets");
            if (assets != null) {
                for (int i = 0; i < assets.length(); i++) {
                    JSONObject a = assets.getJSONObject(i);
                    if (a.optString("name").toLowerCase().endsWith(".apk")) {
                        o.put("apk", a.optString("browser_download_url"));
                        o.put("tamanho", a.optLong("size"));
                        break;
                    }
                }
            }
            return o.toString();
        } catch (Throwable t) {
            return "ERRO:" + mensagemDe(t);
        } finally {
            if (c != null) c.disconnect();
        }
    }

    private static File arquivoAtualizacao(Context c) {
        File pasta = new File(c.getCacheDir(), "atualizacoes");
        pasta.mkdirs();
        return new File(pasta, "AbyssHub.apk");
    }

    public static void baixarAtualizacao(String url) {
        MainActivity a = ativa;
        if (a == null || "baixando".equals(updEstado)) return;
        updEstado = "baixando";
        updProgresso = 0;
        updMsg = "";
        new Thread(() -> {
            HttpURLConnection c = null;
            try {
                c = (HttpURLConnection) new URL(url).openConnection();
                c.setRequestProperty("User-Agent", "AbyssHub");
                c.setConnectTimeout(15000);
                c.setReadTimeout(30000);
                long total = c.getContentLengthLong();
                File f = arquivoAtualizacao(a);
                try (InputStream in = c.getInputStream(); OutputStream out = new FileOutputStream(f)) {
                    byte[] buf = new byte[64 * 1024];
                    long lido = 0;
                    int n;
                    while ((n = in.read(buf)) > 0) {
                        out.write(buf, 0, n);
                        lido += n;
                        if (total > 0) updProgresso = (int) (lido * 100 / total);
                    }
                }
                updProgresso = 100;
                updEstado = "baixado";
                instalarAtualizacao();
            } catch (Throwable t) {
                updEstado = "erro";
                updMsg = mensagemDe(t);
            } finally {
                if (c != null) c.disconnect();
            }
        }).start();
    }

    private static boolean podeInstalar(Context c) {
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.O || c.getPackageManager().canRequestPackageInstalls();
    }

    public static void instalarAtualizacao() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            try {
                File f = arquivoAtualizacao(a);
                if (!f.exists()) {
                    updEstado = "erro";
                    updMsg = "O arquivo da atualização sumiu. Baixe de novo.";
                    return;
                }
                if (!podeInstalar(a)) {
                    updEstado = "permissao";
                    a.startActivity(new Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                            Uri.parse("package:" + a.getPackageName())));
                    return;
                }
                Uri u = FileProvider.getUriForFile(a, a.getPackageName() + ".arquivos", f);
                Intent i = new Intent(Intent.ACTION_VIEW);
                i.setDataAndType(u, "application/vnd.android.package-archive");
                i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_ACTIVITY_NEW_TASK);
                a.startActivity(i);
                updEstado = "instalando";
            } catch (Throwable t) {
                updEstado = "erro";
                updMsg = mensagemDe(t);
            }
        });
    }

    /** JSON: {"estado", "progresso", "msg"} */
    public static String estadoAtualizacao() {
        JSONObject o = new JSONObject();
        try {
            o.put("estado", updEstado);
            o.put("progresso", updProgresso);
            o.put("msg", updMsg);
        } catch (Throwable ignored) {
        }
        return o.toString();
    }

    // =================================================================
    // Leaderboard (Firestore) — chamado de uma thread do Rust, pode bloquear
    // =================================================================

    /** Grava o tempo de jogo da pessoa. Devolve "" se deu certo ou a mensagem de erro. */
    public static String publicar(String json) {
        try {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return "Sem conta";
            JSONObject p = new JSONObject(json);
            FirebaseFirestore db = FirebaseFirestore.getInstance();
            String uid = u.getUid();
            String nome = nomeDe(u);
            String foto = fotoDe(u);
            long agora = System.currentTimeMillis();
            String dia = p.optString("dia");

            WriteBatch b = db.batch();
            Map<String, Object> pessoa = new HashMap<>();
            pessoa.put("uid", uid);
            pessoa.put("nome", nome);
            pessoa.put("foto", foto);
            pessoa.put("dia", dia);
            pessoa.put("atualizado_ms", agora);
            for (String per : new String[]{"hoje", "semana", "tudo"}) {
                pessoa.put(per + "_ms", p.optLong(per + "_ms"));
                JSONObject top = p.optJSONObject("top_" + per);
                pessoa.put(per + "_top_pacote", top != null ? top.optString("pacote") : "");
                pessoa.put(per + "_top_nome", top != null ? top.optString("nome") : "");
            }
            b.set(db.collection(PESSOAS).document(uid), pessoa);

            JSONArray jogos = p.optJSONArray("jogos");
            if (jogos != null) {
                for (int i = 0; i < jogos.length(); i++) {
                    JSONObject j = jogos.getJSONObject(i);
                    String pacote = j.optString("pacote");
                    if (pacote.isEmpty()) continue;
                    Map<String, Object> par = new HashMap<>();
                    par.put("uid", uid);
                    par.put("nome", nome);
                    par.put("foto", foto);
                    par.put("pacote", pacote);
                    par.put("jogo", j.optString("nome"));
                    par.put("dia", dia);
                    par.put("atualizado_ms", agora);
                    par.put("hoje_ms", j.optLong("hoje_ms"));
                    par.put("semana_ms", j.optLong("semana_ms"));
                    par.put("tudo_ms", j.optLong("tudo_ms"));
                    b.set(db.collection(PARES).document(uid + "__" + pacote), par);
                }
            }
            Tasks.await(b.commit(), 20, TimeUnit.SECONDS);
            if (jogos != null) enviarIcones(db, jogos);
            return "";
        } catch (Throwable t) {
            return mensagemDe(t);
        }
    }

    /** Sobe o ícone dos jogos que ainda não estão no Firebase (para quem não tem o jogo ver). */
    private static void enviarIcones(FirebaseFirestore db, JSONArray jogos) {
        MainActivity a = ativa;
        if (a == null) return;
        SharedPreferences prefs = a.getSharedPreferences("abysshub", MODE_PRIVATE);
        Set<String> enviados = new HashSet<>(prefs.getStringSet("icones", new HashSet<>()));
        int feitos = 0;
        try {
            for (int i = 0; i < jogos.length() && feitos < 5; i++) {
                String pacote = jogos.getJSONObject(i).optString("pacote");
                if (pacote.isEmpty() || enviados.contains(pacote)) continue;
                feitos++;
                DocumentReference d = db.collection(ICONES).document(pacote);
                DocumentSnapshot s = Tasks.await(d.get(), 10, TimeUnit.SECONDS);
                if (!s.exists()) {
                    String png = iconePng(pacote, 64);
                    if (png != null) {
                        Map<String, Object> m = new HashMap<>();
                        m.put("nome", jogos.getJSONObject(i).optString("nome"));
                        m.put("png", png);
                        Tasks.await(d.set(m), 10, TimeUnit.SECONDS);
                    }
                }
                enviados.add(pacote);
            }
        } catch (Throwable t) {
            Log.w(TAG, "icones", t);
        }
        prefs.edit().putStringSet("icones", enviados).apply();
    }

    /**
     * modo: "pessoas" | "pares" | "jogo"; periodo: "hoje" | "semana" | "tudo".
     * Devolve um JSON (lista) ou "ERRO:mensagem".
     */
    public static String buscar(String modo, String periodo, String pacote, String dia) {
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) return "ERRO:Sem conta";
            FirebaseFirestore db = FirebaseFirestore.getInstance();
            String campo = periodo + "_ms";
            Query q = db.collection(modo.equals("pessoas") ? PESSOAS : PARES);
            if (modo.equals("jogo")) q = q.whereEqualTo("pacote", pacote);
            if (periodo.equals("hoje")) q = q.whereEqualTo("dia", dia);
            q = q.orderBy(campo, Query.Direction.DESCENDING).limit(60);
            QuerySnapshot snap = Tasks.await(q.get(), 20, TimeUnit.SECONDS);

            long agora = System.currentTimeMillis();
            JSONArray saida = new JSONArray();
            for (DocumentSnapshot d : snap.getDocuments()) {
                long ms = numero(d, campo);
                long atualizado = numero(d, "atualizado_ms");
                if (ms <= 0) continue;
                // quem não abre o app há mais de 7 dias não conta na semana
                if (periodo.equals("semana") && agora - atualizado > SEMANA_MS) continue;
                JSONObject o = new JSONObject();
                o.put("uid", texto(d, "uid"));
                o.put("nome", texto(d, "nome"));
                o.put("foto", texto(d, "foto"));
                o.put("ms", ms);
                o.put("atualizado_ms", atualizado);
                if (modo.equals("pessoas")) {
                    o.put("pacote", texto(d, periodo + "_top_pacote"));
                    o.put("jogo", texto(d, periodo + "_top_nome"));
                } else {
                    o.put("pacote", texto(d, "pacote"));
                    o.put("jogo", texto(d, "jogo"));
                }
                saida.put(o);
                if (saida.length() >= 50) break;
            }
            return saida.toString();
        } catch (Throwable t) {
            return "ERRO:" + mensagemDe(t);
        }
    }

    /** Tira a pessoa do Leaderboard. Devolve "" se deu certo. */
    public static String apagarMeusDados() {
        try {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return "Sem conta";
            FirebaseFirestore db = FirebaseFirestore.getInstance();
            QuerySnapshot pares = Tasks.await(
                    db.collection(PARES).whereEqualTo("uid", u.getUid()).get(), 20, TimeUnit.SECONDS);
            WriteBatch b = db.batch();
            for (DocumentSnapshot d : pares.getDocuments()) b.delete(d.getReference());
            b.delete(db.collection(PESSOAS).document(u.getUid()));
            Tasks.await(b.commit(), 20, TimeUnit.SECONDS);
            return "";
        } catch (Throwable t) {
            return mensagemDe(t);
        }
    }

    private static long numero(DocumentSnapshot d, String k) {
        Object o = d.get(k);
        return o instanceof Number ? ((Number) o).longValue() : 0L;
    }

    private static String texto(DocumentSnapshot d, String k) {
        Object o = d.get(k);
        return o instanceof String ? (String) o : "";
    }

    private static String mensagemDe(Throwable t) {
        Throwable c = t;
        while (c.getCause() != null && c.getCause() != c) c = c.getCause();
        String m = c.getMessage();
        return m == null ? c.getClass().getSimpleName() : m;
    }

    // =================================================================
    // Imagens — devolvem pixels RGBA (pré-multiplicados), tam x tam
    // =================================================================

    /** Ícone de um app: do próprio celular ou, se não tiver instalado, do Firebase. */
    public static byte[] iconeApp(String pacote, int tam) {
        byte[] local = iconeLocal(pacote, tam);
        if (local != null) return local;
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) return null;
            DocumentSnapshot s = Tasks.await(
                    FirebaseFirestore.getInstance().collection(ICONES).document(pacote).get(),
                    10, TimeUnit.SECONDS);
            Object o = s.get("png");
            if (!(o instanceof String)) return null;
            String png = (String) o;
            byte[] bytes = Base64.decode(png, Base64.DEFAULT);
            Bitmap b = BitmapFactory.decodeByteArray(bytes, 0, bytes.length);
            return b == null ? null : paraRgba(b, tam);
        } catch (Throwable t) {
            return null;
        }
    }

    /** Baixa uma foto (ex.: a foto da conta Google). */
    public static byte[] baixarImagem(String url, int tam) {
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL(url).openConnection();
            c.setConnectTimeout(10000);
            c.setReadTimeout(10000);
            try (InputStream in = c.getInputStream()) {
                Bitmap b = BitmapFactory.decodeStream(in);
                return b == null ? null : paraRgba(b, tam);
            }
        } catch (Throwable t) {
            return null;
        } finally {
            if (c != null) c.disconnect();
        }
    }

    private static Bitmap bitmapDoIcone(String pacote, int tam) {
        MainActivity a = ativa;
        if (a == null) return null;
        try {
            Drawable d = a.getPackageManager().getApplicationIcon(pacote);
            Bitmap b = Bitmap.createBitmap(tam, tam, Bitmap.Config.ARGB_8888);
            Canvas c = new Canvas(b);
            d.setBounds(0, 0, tam, tam);
            d.draw(c);
            return b;
        } catch (PackageManager.NameNotFoundException e) {
            return null;
        } catch (Throwable t) {
            return null;
        }
    }

    private static byte[] iconeLocal(String pacote, int tam) {
        Bitmap b = bitmapDoIcone(pacote, tam);
        return b == null ? null : paraRgba(b, tam);
    }

    private static String iconePng(String pacote, int tam) {
        Bitmap b = bitmapDoIcone(pacote, tam);
        if (b == null) return null;
        ByteArrayOutputStream s = new ByteArrayOutputStream();
        b.compress(Bitmap.CompressFormat.PNG, 100, s);
        return Base64.encodeToString(s.toByteArray(), Base64.NO_WRAP);
    }

    /** Recorta no meio, redimensiona e devolve os bytes RGBA. */
    private static byte[] paraRgba(Bitmap origem, int tam) {
        Bitmap b = Bitmap.createBitmap(tam, tam, Bitmap.Config.ARGB_8888);
        Canvas c = new Canvas(b);
        int w = origem.getWidth(), h = origem.getHeight(), lado = Math.min(w, h);
        Rect src = new Rect((w - lado) / 2, (h - lado) / 2, (w - lado) / 2 + lado, (h - lado) / 2 + lado);
        c.drawBitmap(origem, src, new Rect(0, 0, tam, tam), new Paint(Paint.FILTER_BITMAP_FLAG | Paint.ANTI_ALIAS_FLAG));
        ByteBuffer buf = ByteBuffer.allocate(tam * tam * 4);
        b.copyPixelsToBuffer(buf);
        return buf.array();
    }
}
