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
import com.google.firebase.firestore.FieldPath;
import com.google.firebase.firestore.ListenerRegistration;
import com.google.firebase.firestore.FirebaseFirestore;
import com.google.firebase.firestore.Query;
import com.google.firebase.firestore.QuerySnapshot;
import com.google.firebase.firestore.WriteBatch;
import com.google.firebase.functions.FirebaseFunctions;
import com.google.firebase.messaging.FirebaseMessaging;
import com.google.firebase.functions.FirebaseFunctionsException;
import com.google.firebase.functions.HttpsCallableResult;

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
import java.util.ArrayList;
import java.util.Calendar;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
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
    private static final String HISTORICO = "abysshub_historico";
    private static final String EVENTOS = "abysshub_eventos";
    private static final String ADMINS = "abysshub_admins";
    private static final String ICONES = "abysshub_icones";
    private static final long SEMANA_MS = 7L * 24 * 60 * 60 * 1000;

    static volatile MainActivity ativa;
    /** Contexto do app (continua valendo mesmo se a tela for recriada). */
    private static volatile Context contexto;
    /** "saiu", "entrando" ou "erro" (logado vem do próprio FirebaseAuth) */
    private static volatile String estado = "saiu";
    private static volatile String mensagem = "";

    private static final Executor PRINCIPAL = r -> new Handler(Looper.getMainLooper()).post(r);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        ativa = this;
        contexto = getApplicationContext();
        super.onCreate(savedInstanceState);
        prepararAvisos();
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
                garantirPerfil(u.getUid());
                checarCriador(u.getUid());
                o.put("criador", criador && u.getUid().equals(criadorUid));
                ouvirMolduras(u.getUid());
                o.put("moldura", molduraEfetiva(u));
                o.put("molduras", new JSONArray(moldurasTem));
                o.put("estado", "logado");
                o.put("uid", u.getUid());
                o.put("nome", nomeEfetivo(u));
                o.put("foto", fotoEfetiva(u));
                o.put("email", u.getEmail() == null ? "" : u.getEmail());
                JSONArray prov = new JSONArray();
                for (com.google.firebase.auth.UserInfo i : u.getProviderData()) {
                    if (!"firebase".equals(i.getProviderId())) prov.put(i.getProviderId());
                }
                o.put("provedores", prov);
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
        if (n == null || n.trim().isEmpty()) {
            for (com.google.firebase.auth.UserInfo i : u.getProviderData()) {
                if (i.getDisplayName() != null && !i.getDisplayName().trim().isEmpty()) {
                    n = i.getDisplayName();
                    break;
                }
            }
        }
        if (n == null || n.trim().isEmpty()) n = "Jogador";
        return n.trim();
    }

    /** Foto da conta Google (também quando o Google foi conectado depois a uma conta de e-mail). */
    private static String fotoDe(FirebaseUser u) {
        Uri f = u.getPhotoUrl();
        if (f == null) {
            for (com.google.firebase.auth.UserInfo i : u.getProviderData()) {
                if ("google.com".equals(i.getProviderId()) && i.getPhotoUrl() != null) {
                    f = i.getPhotoUrl();
                    break;
                }
            }
        }
        if (f == null) return "";
        // foto maior que a padrão (96 px)
        return f.toString().replace("s96-c", "s256-c");
    }

    // =================================================================
    // Perfil do Leaderboard: nome e foto escolhidos pela pessoa
    // (abysshub_perfis/{uid}: nome, foto_ver; a foto fica em abysshub_fotos/{uid})
    // =================================================================

    private static final String PERFIS = "abysshub_perfis";
    private static final String FOTOS = "abysshub_fotos";
    private static final int PEDIDO_FOTO = 4107;

    private static volatile String perfilUid = "";
    private static volatile boolean perfilCarregando = false;
    private static volatile String perfilNome = "";
    /** 0 = foto do Google, -1 = sem foto, maior que 0 = foto escolhida (versão) */
    private static volatile long perfilFotoVer = 0;
    /** Moldura que a pessoa está usando ("" = nenhuma). */
    private static volatile String perfilMoldura = "";

    // Molduras que a pessoa tem (abysshub_molduras/{uid}, só o servidor escreve)
    private static final String MOLDURAS = "abysshub_molduras";
    private static volatile List<String> moldurasTem = new ArrayList<>();
    private static volatile String moldurasUid = "";
    private static ListenerRegistration moldurasEscuta = null;

    private static String molduraEfetiva(FirebaseUser u) {
        return perfilUid.equals(u.getUid()) ? perfilMoldura : "";
    }

    /** Fica ouvindo as molduras da conta (chegam sozinhas quando a pessoa ganha um evento). */
    private static synchronized void ouvirMolduras(String uid) {
        if (uid.equals(moldurasUid)) return;
        if (moldurasEscuta != null) moldurasEscuta.remove();
        moldurasUid = uid;
        moldurasTem = new ArrayList<>();
        try {
            moldurasEscuta = FirebaseFirestore.getInstance().collection(MOLDURAS).document(uid)
                    .addSnapshotListener((d, e) -> {
                        if (d == null || !uid.equals(moldurasUid)) return;
                        List<String> v = new ArrayList<>();
                        Object tem = d.get("tem");
                        if (tem instanceof List) for (Object x : (List<?>) tem) if (x instanceof String) v.add((String) x);
                        moldurasTem = v;
                    });
        } catch (Throwable t) {
            Log.w(TAG, "molduras", t);
        }
    }

    private static synchronized void pararMolduras() {
        if (moldurasEscuta != null) moldurasEscuta.remove();
        moldurasEscuta = null;
        moldurasUid = "";
        moldurasTem = new ArrayList<>();
    }

    /** Usa a moldura (precisa ter ganhado). "" = sem moldura. Devolve "" se deu certo. */
    public static String usarMoldura(String id) {
        try {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return "Entre na sua conta primeiro";
            if (!id.isEmpty() && !moldurasTem.contains(id)) return "Você ainda não tem essa moldura";
            if (!perfilUid.equals(u.getUid())) carregarPerfil(u.getUid());
            Map<String, Object> m = new HashMap<>();
            m.put("moldura", id);
            Tasks.await(FirebaseFirestore.getInstance().collection(PERFIS).document(u.getUid())
                    .set(m, com.google.firebase.firestore.SetOptions.merge()), 20, TimeUnit.SECONDS);
            perfilMoldura = id;
            return "";
        } catch (Throwable t) {
            String msg = mensagemDe(t);
            if (msg.contains("PERMISSION_DENIED")) return "Você ainda não tem essa moldura";
            return msg;
        }
    }

    private static String nomeEfetivo(FirebaseUser u) {
        return perfilUid.equals(u.getUid()) && !perfilNome.isEmpty() ? perfilNome : nomeDe(u);
    }

    private static String fotoEfetiva(FirebaseUser u) {
        if (perfilUid.equals(u.getUid())) {
            if (perfilFotoVer > 0) return "fs:" + u.getUid() + ":" + perfilFotoVer;
            if (perfilFotoVer < 0) return "";
        }
        return fotoDe(u);
    }

    /** Lê o perfil salvo (uma vez por conta), numa thread. */
    private static void garantirPerfil(String uid) {
        if (uid.equals(perfilUid) || perfilCarregando) return;
        perfilCarregando = true;
        new Thread(() -> {
            try {
                carregarPerfil(uid);
            } finally {
                perfilCarregando = false;
            }
        }).start();
    }

    private static void carregarPerfil(String uid) {
        try {
            DocumentSnapshot d = Tasks.await(
                    FirebaseFirestore.getInstance().collection(PERFIS).document(uid).get(), 15, TimeUnit.SECONDS);
            perfilNome = texto(d, "nome").trim();
            perfilFotoVer = numero(d, "foto_ver");
            perfilMoldura = texto(d, "moldura");
            perfilUid = uid;
        } catch (Throwable t) {
            Log.w(TAG, "perfil", t);
        }
    }

    private static void salvarPerfil(String uid, Map<String, Object> campos, String ok) {
        FirebaseFirestore.getInstance().collection(PERFIS).document(uid)
                .set(campos, com.google.firebase.firestore.SetOptions.merge())
                .addOnCompleteListener(t -> {
                    if (t.isSuccessful()) {
                        if (ok != null) avisar(ok);
                    } else {
                        avisar("Não foi possível salvar: " + mensagemDe(t.getException()));
                    }
                });
    }

    /** Janelinha para mudar o nome que aparece no Leaderboard. */
    public static void editarNome() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return;
            EditText campo = a.campo("Seu nome", InputType.TYPE_CLASS_TEXT
                    | InputType.TYPE_TEXT_VARIATION_PERSON_NAME | InputType.TYPE_TEXT_FLAG_CAP_WORDS);
            campo.setText(nomeEfetivo(u));
            campo.setSelection(campo.getText().length());
            LinearLayout l = new LinearLayout(a);
            l.setPadding(a.dp(22), a.dp(8), a.dp(22), 0);
            l.addView(campo, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT));
            AlertDialog d = new AlertDialog.Builder(a, a.temaDialogo())
                    .setTitle("Nome no Leaderboard")
                    .setView(l)
                    .setPositiveButton("Salvar", null)
                    .setNegativeButton("Cancelar", null)
                    .create();
            d.setOnShowListener(x -> d.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener(v -> {
                String n = campo.getText().toString().trim();
                if (n.isEmpty()) { campo.setError("Digite um nome"); return; }
                if (n.length() > 30) { campo.setError("Use no máximo 30 letras"); return; }
                perfilUid = u.getUid();
                perfilNome = n;
                Map<String, Object> m = new HashMap<>();
                m.put("nome", n);
                salvarPerfil(u.getUid(), m, "Nome atualizado!");
                d.dismiss();
            }));
            d.show();
        });
    }

    /** Escolher a foto do Leaderboard: da galeria, a do Google ou nenhuma. */
    public static void editarFoto() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return;
            boolean temGoogle = !fotoDe(u).isEmpty();
            String[] opcoes = temGoogle
                    ? new String[]{"Escolher da galeria", "Usar a foto da conta Google", "Ficar sem foto"}
                    : new String[]{"Escolher da galeria", "Ficar sem foto"};
            new AlertDialog.Builder(a, a.temaDialogo())
                    .setTitle("Foto no Leaderboard")
                    .setItems(opcoes, (d, i) -> {
                        String escolha = opcoes[i];
                        Map<String, Object> m = new HashMap<>();
                        if (escolha.startsWith("Escolher")) {
                            Intent pegar = new Intent(Intent.ACTION_GET_CONTENT).setType("image/*")
                                    .addCategory(Intent.CATEGORY_OPENABLE);
                            a.startActivityForResult(Intent.createChooser(pegar, "Escolha uma foto"), PEDIDO_FOTO);
                            return;
                        }
                        perfilUid = u.getUid();
                        perfilFotoVer = escolha.startsWith("Usar") ? 0 : -1;
                        m.put("foto_ver", perfilFotoVer);
                        salvarPerfil(u.getUid(), m, "Foto atualizada!");
                    })
                    .setNegativeButton("Cancelar", null)
                    .show();
        });
    }

    @Override
    protected void onActivityResult(int pedido, int resultado, Intent dados) {
        super.onActivityResult(pedido, resultado, dados);
        if (pedido != PEDIDO_FOTO || resultado != RESULT_OK || dados == null || dados.getData() == null) return;
        Uri origem = dados.getData();
        FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
        if (u == null) return;
        avisar("Enviando a foto…");
        new Thread(() -> {
            try {
                // lê a imagem já reduzida (fotos de câmera são enormes)
                BitmapFactory.Options op = new BitmapFactory.Options();
                op.inJustDecodeBounds = true;
                try (InputStream in = getContentResolver().openInputStream(origem)) {
                    BitmapFactory.decodeStream(in, null, op);
                }
                int amostra = 1;
                while (Math.min(op.outWidth, op.outHeight) / (amostra * 2) >= 256) amostra *= 2;
                BitmapFactory.Options op2 = new BitmapFactory.Options();
                op2.inSampleSize = amostra;
                Bitmap b;
                try (InputStream in = getContentResolver().openInputStream(origem)) {
                    b = BitmapFactory.decodeStream(in, null, op2);
                }
                if (b == null) throw new IllegalStateException("não deu para abrir essa imagem");
                Bitmap quadrada = Bitmap.createBitmap(256, 256, Bitmap.Config.ARGB_8888);
                Canvas c = new Canvas(quadrada);
                c.drawColor(0xFF12183E);
                int w = b.getWidth(), h = b.getHeight(), lado = Math.min(w, h);
                Rect src = new Rect((w - lado) / 2, (h - lado) / 2, (w - lado) / 2 + lado, (h - lado) / 2 + lado);
                c.drawBitmap(b, src, new Rect(0, 0, 256, 256), new Paint(Paint.FILTER_BITMAP_FLAG | Paint.ANTI_ALIAS_FLAG));
                ByteArrayOutputStream s = new ByteArrayOutputStream();
                quadrada.compress(Bitmap.CompressFormat.JPEG, 82, s);
                String jpg = Base64.encodeToString(s.toByteArray(), Base64.NO_WRAP);

                long ver = System.currentTimeMillis();
                Map<String, Object> foto = new HashMap<>();
                foto.put("jpg", jpg);
                foto.put("ver", ver);
                Tasks.await(FirebaseFirestore.getInstance().collection(FOTOS).document(u.getUid()).set(foto),
                        30, TimeUnit.SECONDS);
                perfilUid = u.getUid();
                perfilFotoVer = ver;
                Map<String, Object> m = new HashMap<>();
                m.put("foto_ver", ver);
                salvarPerfil(u.getUid(), m, "Foto atualizada!");
            } catch (Throwable t) {
                avisar("Não foi possível usar essa foto: " + mensagemDe(t));
            }
        }).start();
    }

    /** Adiciona e-mail e senha a quem entrou com Google (para entrar das duas formas). */
    public static void ligarEmail() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return;
            LinearLayout l = new LinearLayout(a);
            l.setOrientation(LinearLayout.VERTICAL);
            l.setPadding(a.dp(22), a.dp(8), a.dp(22), 0);
            EditText email = a.campo("E-mail", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS);
            if (u.getEmail() != null) email.setText(u.getEmail());
            EditText senha = a.campo("Crie uma senha (mínimo 6 caracteres)",
                    InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD);
            l.addView(email);
            l.addView(senha);
            AlertDialog d = new AlertDialog.Builder(a, a.temaDialogo())
                    .setTitle("Adicionar e-mail e senha")
                    .setView(l)
                    .setPositiveButton("Adicionar", null)
                    .setNegativeButton("Cancelar", null)
                    .create();
            d.setOnShowListener(x -> d.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener(v -> {
                String e = email.getText().toString().trim();
                String s = senha.getText().toString();
                if (e.isEmpty()) { email.setError("Digite seu e-mail"); return; }
                if (s.length() < 6) { senha.setError("A senha precisa de pelo menos 6 caracteres"); return; }
                v.setEnabled(false);
                u.linkWithCredential(com.google.firebase.auth.EmailAuthProvider.getCredential(e, s))
                        .addOnCompleteListener(t -> {
                            if (t.isSuccessful()) {
                                avisar("Pronto! Agora você também entra com " + e + ".");
                                d.dismiss();
                            } else {
                                v.setEnabled(true);
                                Exception ex = t.getException();
                                avisar(ex instanceof FirebaseAuthUserCollisionException
                                        ? "Esse e-mail já é usado por outro perfil do AbyssHub."
                                        : "Não foi possível adicionar: " + traduzir(ex));
                            }
                        });
            }));
            d.show();
        });
    }

    /** Abre a janelinha de contas do Google. O resultado aparece em estadoConta(). */
    public static void entrar() {
        pedirGoogle(false);
    }

    /** Liga uma conta Google à conta atual (quem entrou com e-mail e senha). */
    public static void ligarGoogle() {
        pedirGoogle(true);
    }

    private static void pedirGoogle(boolean ligar) {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            if (!firebasePronto(a)) {
                estado = "erro";
                mensagem = "O login não está disponível nesta versão do app.";
                return;
            }
            if (!ligar) {
                estado = "entrando";
                mensagem = "";
            }
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
                                usarCredencial(r.getCredential(), ligar);
                            }

                            @Override
                            public void onError(GetCredentialException e) {
                                if (e instanceof GetCredentialCancellationException) {
                                    if (!ligar) {
                                        estado = "saiu";
                                        mensagem = "";
                                    }
                                } else if (e instanceof NoCredentialException) {
                                    falhou("Nenhuma conta Google neste celular.", ligar);
                                } else {
                                    falhou("Não foi possível usar o Google: " + e.getMessage(), ligar);
                                }
                            }
                        });
            } catch (Throwable t) {
                falhou("Não foi possível usar o Google: " + t.getMessage(), ligar);
            }
        });
    }

    private static void usarCredencial(Credential c, boolean ligar) {
        try {
            if (c instanceof CustomCredential
                    && GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL.equals(c.getType())) {
                GoogleIdTokenCredential g = GoogleIdTokenCredential.createFrom(c.getData());
                AuthCredential cred = GoogleAuthProvider.getCredential(g.getIdToken(), null);
                FirebaseUser atual = FirebaseAuth.getInstance().getCurrentUser();
                if (ligar && atual != null) {
                    atual.linkWithCredential(cred).addOnCompleteListener(t -> {
                        if (t.isSuccessful()) {
                            avisar("Conta Google conectada! Agora dá para entrar com ela também.");
                        } else if (t.getException() instanceof FirebaseAuthUserCollisionException) {
                            avisar("Essa conta Google já é usada por outro perfil do AbyssHub.");
                        } else {
                            avisar("Não foi possível conectar: " + traduzir(t.getException()));
                        }
                    });
                    return;
                }
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
                falhou("Tipo de conta não suportado.", ligar);
            }
        } catch (Throwable t) {
            falhou("Erro no login: " + t.getMessage(), ligar);
        }
    }

    private static void falhou(String msg, boolean soAvisar) {
        if (soAvisar) avisar(msg);
        else falhou(msg);
    }

    /** Mensagem rápida na tela (Toast). */
    private static void avisar(String msg) {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> Toast.makeText(a, msg, Toast.LENGTH_LONG).show());
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
        perfilUid = "";
        perfilNome = "";
        perfilFotoVer = 0;
        perfilMoldura = "";
        pararMolduras();
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

    private int temaDialogo() {
        return escuro() ? android.R.style.Theme_DeviceDefault_Dialog_Alert
                : android.R.style.Theme_DeviceDefault_Light_Dialog_Alert;
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

        AlertDialog d = new AlertDialog.Builder(this, temaDialogo())
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
    // Steam — cada pessoa usa a própria chave da Steam Web API.
    // A chave fica só neste celular (SharedPreferences "abysshub_steam"), nunca vai ao Firebase.
    // =================================================================

    private static final String STEAM_API = "https://api.steampowered.com";
    private static final String STEAM_CHAVE_URL = "https://steamcommunity.com/dev/apikey";

    /** "" = ver as preferências; "conectando"; "erro" (com steamMsg). */
    private static volatile String steamEstado = "";
    private static volatile String steamMsg = "";

    private static SharedPreferences prefsSteam() {
        Context c = contexto;
        return c == null ? null : c.getSharedPreferences("abysshub_steam", MODE_PRIVATE);
    }

    /** {"estado": "desligado"|"conectando"|"ok"|"erro", "msg", "nome"} */
    public static String steamEstado() {
        try {
            JSONObject o = new JSONObject();
            SharedPreferences p = prefsSteam();
            boolean conectado = p != null && !p.getString("id", "").isEmpty() && !p.getString("chave", "").isEmpty();
            String e = steamEstado;
            if (e.equals("conectando")) o.put("estado", "conectando");
            else if (e.equals("erro")) o.put("estado", "erro");
            else o.put("estado", conectado ? "ok" : "desligado");
            o.put("msg", steamMsg);
            o.put("nome", p == null ? "" : p.getString("nome", ""));
            return o.toString();
        } catch (Throwable t) {
            return "{\"estado\":\"desligado\"}";
        }
    }

    public static void steamDesconectar() {
        SharedPreferences p = prefsSteam();
        if (p != null) p.edit().clear().apply();
        steamEstado = "";
        steamMsg = "";
    }

    public static void steamConectar() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(a::dialogoSteam);
    }

    private void dialogoSteam() {
        LinearLayout l = new LinearLayout(this);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setPadding(dp(22), dp(8), dp(22), 0);
        TextView passos = new TextView(this);
        passos.setText("1. Toque em \"Pegar minha chave\" e entre na sua conta Steam. Em \"Nome de domínio\" "
                + "escreva qualquer coisa (por exemplo: abysshub), aceite os termos e copie a chave.\n"
                + "2. Cole a chave aqui embaixo.\n"
                + "3. Cole o link do seu perfil da Steam (ou o seu ID).\n\n"
                + "Na Steam, em Perfil > Editar perfil > Privacidade, deixe \"Detalhes do jogo\" como Público.\n"
                + "A chave fica guardada só neste celular.");
        passos.setTextSize(13);
        passos.setPadding(0, 0, 0, dp(8));
        l.addView(passos);
        SharedPreferences p = prefsSteam();
        final EditText perfil = campo("Link do perfil ou ID da Steam", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
        final EditText chave = campo("Chave da Steam (32 letras e números)",
                InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD);
        if (p != null) {
            perfil.setText(p.getString("perfil", ""));
            chave.setText(p.getString("chave", ""));
        }
        l.addView(perfil);
        l.addView(chave);

        AlertDialog d = new AlertDialog.Builder(this, temaDialogo())
                .setTitle("Conectar Steam")
                .setView(l)
                .setPositiveButton("Conectar", null)
                .setNeutralButton("Pegar minha chave", null)
                .setNegativeButton("Cancelar", null)
                .create();
        d.setOnShowListener(x -> {
            d.getButton(DialogInterface.BUTTON_NEUTRAL).setOnClickListener(v -> {
                try {
                    startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(STEAM_CHAVE_URL)));
                } catch (Throwable t) {
                    Toast.makeText(this, "Abra " + STEAM_CHAVE_URL + " no navegador", Toast.LENGTH_LONG).show();
                }
            });
            d.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener(v -> {
                String pf = perfil.getText().toString().trim();
                String ch = chave.getText().toString().trim().replace(" ", "");
                if (pf.isEmpty()) { perfil.setError("Cole o link do seu perfil"); return; }
                if (!ch.matches("[0-9A-Fa-f]{32}")) { chave.setError("A chave tem 32 letras e números"); return; }
                d.dismiss();
                steamEstado = "conectando";
                steamMsg = "";
                new Thread(() -> validarSteam(pf, ch)).start();
            });
        });
        d.show();
    }

    /** Confere o perfil e a chave e, se der certo, guarda. Roda numa thread. */
    private static void validarSteam(String perfil, String chave) {
        try {
            String id = resolverSteamId(perfil, chave);
            JSONObject resp = donoDosJogos(id, chave);
            int n = resp.optInt("game_count", 0);
            String nome = "";
            try {
                JSONObject r = new JSONObject(httpTexto(STEAM_API + "/ISteamUser/GetPlayerSummaries/v2/?key="
                        + chave + "&steamids=" + id));
                JSONArray ps = r.getJSONObject("response").optJSONArray("players");
                if (ps != null && ps.length() > 0) nome = ps.getJSONObject(0).optString("personaname");
            } catch (Throwable ignorar) { }
            SharedPreferences p = prefsSteam();
            if (p == null) throw new Exception("O app foi fechado");
            p.edit().putString("perfil", perfil).putString("chave", chave)
                    .putString("id", id).putString("nome", nome).apply();
            steamMsg = "";
            steamEstado = "";
            Log.i(TAG, "Steam conectada: " + n + " jogos");
        } catch (Throwable t) {
            steamMsg = mensagemDe(t);
            steamEstado = "erro";
            avisar(steamMsg);
        }
    }

    /** Aceita link /profiles/123..., link /id/nome, o ID de 17 dígitos ou só o nome personalizado. */
    private static String resolverSteamId(String entrada, String chave) throws Exception {
        String e = entrada.trim();
        int i = e.indexOf("/profiles/");
        if (i >= 0) {
            String resto = e.substring(i + 10).replaceAll("[^0-9].*$", "");
            if (resto.matches("\\d{17}")) return resto;
        }
        if (e.matches("\\d{17}")) return e;
        String nome = e;
        i = e.indexOf("/id/");
        if (i >= 0) nome = e.substring(i + 4);
        nome = nome.replaceAll("[/?#].*$", "");
        if (nome.isEmpty()) throw new Exception("Não entendi esse link de perfil da Steam.");
        JSONObject r = new JSONObject(httpTexto(STEAM_API + "/ISteamUser/ResolveVanityURL/v1/?key=" + chave
                + "&vanityurl=" + Uri.encode(nome))).getJSONObject("response");
        if (r.optInt("success") != 1) throw new Exception("Não achei esse perfil da Steam. Confira o link.");
        return r.getString("steamid");
    }

    /** O "response" do GetOwnedGames. Dá erro se o perfil estiver privado. */
    private static JSONObject donoDosJogos(String id, String chave) throws Exception {
        JSONObject r = new JSONObject(httpTexto(STEAM_API + "/IPlayerService/GetOwnedGames/v1/?key=" + chave
                + "&steamid=" + id + "&include_appinfo=1&include_played_free_games=1&format=json"))
                .optJSONObject("response");
        if (r == null || !r.has("game_count")) {
            throw new Exception("Os jogos desse perfil estão privados. Na Steam, em Privacidade, deixe \"Detalhes do jogo\" como Público.");
        }
        return r;
    }

    /** Lista os jogos com tempo: {"id", "jogos": [{appid, nome, min, ultima}]}, "DESLIGADO" ou "ERRO:msg". */
    public static String steamJogos() {
        SharedPreferences p = prefsSteam();
        if (p == null) return "ERRO:O app ainda está abrindo";
        String id = p.getString("id", ""), chave = p.getString("chave", "");
        if (id.isEmpty() || chave.isEmpty()) return "DESLIGADO";
        try {
            JSONArray jogos = donoDosJogos(id, chave).optJSONArray("games");
            JSONArray saida = new JSONArray();
            JSONObject icones = new JSONObject(p.getString("icones", "{}"));
            if (jogos != null) {
                for (int i = 0; i < jogos.length(); i++) {
                    JSONObject g = jogos.getJSONObject(i);
                    long min = g.optLong("playtime_forever");
                    if (min <= 0) continue;
                    String appid = String.valueOf(g.optLong("appid"));
                    JSONObject o = new JSONObject();
                    o.put("appid", appid);
                    o.put("nome", g.optString("name"));
                    o.put("min", min);
                    o.put("ultima", g.optLong("rtime_last_played"));
                    saida.put(o);
                    String hash = g.optString("img_icon_url");
                    if (!hash.isEmpty()) icones.put(appid, hash);
                }
            }
            p.edit().putString("icones", icones.toString()).apply();
            JSONObject r = new JSONObject();
            r.put("id", id);
            r.put("jogos", saida);
            return r.toString();
        } catch (Throwable t) {
            return "ERRO:Steam: " + mensagemDe(t);
        }
    }

    /** Arte do jogo da Steam ("steam:{appid}"): capa da biblioteca, cabeçalho ou o ícone pequeno. */
    private static Bitmap bitmapSteam(String pacote) {
        String appid = pacote.substring("steam:".length());
        if (!appid.matches("\\d+")) return null;
        String[] urls = {
                "https://cdn.cloudflare.steamstatic.com/steam/apps/" + appid + "/library_600x900.jpg",
                "https://cdn.cloudflare.steamstatic.com/steam/apps/" + appid + "/header.jpg",
                null,
        };
        SharedPreferences p = prefsSteam();
        if (p != null) {
            try {
                String hash = new JSONObject(p.getString("icones", "{}")).optString(appid);
                if (!hash.isEmpty()) {
                    urls[2] = "https://media.steampowered.com/steamcommunity/public/images/apps/" + appid + "/" + hash + ".jpg";
                }
            } catch (Throwable ignorar) { }
        }
        for (String u : urls) {
            if (u == null) continue;
            Bitmap b = baixarBitmap(u);
            if (b != null) return b;
        }
        return null;
    }

    private static Bitmap baixarBitmap(String url) {
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL(url).openConnection();
            c.setConnectTimeout(10000);
            c.setReadTimeout(10000);
            if (c.getResponseCode() != 200) return null;
            try (InputStream in = c.getInputStream()) {
                return BitmapFactory.decodeStream(in);
            }
        } catch (Throwable t) {
            return null;
        } finally {
            if (c != null) c.disconnect();
        }
    }

    /** GET que devolve o texto da resposta. A chave nunca aparece nas mensagens de erro. */
    private static String httpTexto(String url) throws Exception {
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL(url).openConnection();
            c.setRequestProperty("User-Agent", "AbyssHub");
            c.setConnectTimeout(15000);
            c.setReadTimeout(20000);
            int codigo = c.getResponseCode();
            if (codigo == 401 || codigo == 403) throw new Exception("A Steam recusou a chave. Confira se copiou a chave certa.");
            if (codigo == 429) throw new Exception("A Steam pediu para esperar um pouco. Tente de novo daqui a alguns minutos.");
            if (codigo != 200) throw new Exception("A Steam respondeu com erro " + codigo + ".");
            ByteArrayOutputStream b = new ByteArrayOutputStream();
            try (InputStream in = c.getInputStream()) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) > 0) b.write(buf, 0, n);
            }
            return b.toString("UTF-8");
        } catch (java.net.UnknownHostException e) {
            throw new Exception("Sem internet para falar com a Steam.");
        } finally {
            if (c != null) c.disconnect();
        }
    }

    // =================================================================
    // Histórico por dia (Placar e Eventos)
    // =================================================================

    /** Envia o histórico por dia (JSON montado em historico.rs). Devolve "" se deu certo. */
    public static String publicarHistorico(String json) {
        try {
            FirebaseUser u = FirebaseAuth.getInstance().getCurrentUser();
            if (u == null) return "Sem conta";
            if (!perfilUid.equals(u.getUid())) carregarPerfil(u.getUid());
            Map<String, Object> doc = mapa(new JSONObject(json));
            doc.put("uid", u.getUid());
            doc.put("nome", nomeEfetivo(u));
            doc.put("foto", fotoEfetiva(u));
            doc.put("moldura", molduraEfetiva(u));
            doc.put("atualizado_ms", System.currentTimeMillis());
            Tasks.await(FirebaseFirestore.getInstance().collection(HISTORICO).document(u.getUid()).set(doc),
                    30, TimeUnit.SECONDS);
            return "";
        } catch (Throwable t) {
            return mensagemDe(t);
        }
    }

    /** O histórico de todo mundo (lista em JSON) ou "ERRO:msg". */
    public static String historicos() {
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) return "ERRO:Sem conta";
            QuerySnapshot snap = Tasks.await(
                    FirebaseFirestore.getInstance().collection(HISTORICO).limit(300).get(), 30, TimeUnit.SECONDS);
            JSONArray a = new JSONArray();
            for (DocumentSnapshot d : snap.getDocuments()) {
                Map<String, Object> m = d.getData();
                if (m != null) a.put(new JSONObject(m));
            }
            return a.toString();
        } catch (Throwable t) {
            return "ERRO:" + mensagemDe(t);
        }
    }

    // =================================================================
    // Eventos (só a conta do criador cria, edita e apaga)
    // =================================================================

    /** Aumenta quando um evento é salvo ou apagado, para o Rust recarregar a lista. */
    private static volatile long eventosVersao = 0;
    private static volatile String criadorUid = "";
    private static volatile boolean criador = false;
    private static volatile long criadorChecado = 0;

    /** Vê (de vez em quando) se a conta está em abysshub_admins. */
    private static void checarCriador(String uid) {
        long agora = System.currentTimeMillis();
        if (uid.equals(criadorUid) && agora - criadorChecado < 10 * 60 * 1000) return;
        if (!uid.equals(criadorUid)) criador = false;
        criadorUid = uid;
        criadorChecado = agora;
        try {
            FirebaseFirestore.getInstance().collection(ADMINS).document(uid).get()
                    .addOnSuccessListener(d -> { if (uid.equals(criadorUid)) criador = d.exists(); })
                    .addOnFailureListener(e -> { if (uid.equals(criadorUid)) criador = false; });
        } catch (Throwable t) {
            criador = false;
        }
    }

    public static String eventosVersao() {
        return String.valueOf(eventosVersao);
    }

    /** Lista de eventos (JSON) ou "ERRO:msg". */
    public static String listaEventos() {
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) return "ERRO:Sem conta";
            QuerySnapshot snap = Tasks.await(FirebaseFirestore.getInstance().collection(EVENTOS)
                    .orderBy("inicio_ms", Query.Direction.DESCENDING).limit(100).get(), 20, TimeUnit.SECONDS);
            JSONArray a = new JSONArray();
            for (DocumentSnapshot d : snap.getDocuments()) {
                Map<String, Object> m = d.getData();
                if (m == null) continue;
                JSONObject o = new JSONObject(m);
                o.put("id", d.getId());
                a.put(o);
            }
            return a.toString();
        } catch (Throwable t) {
            return "ERRO:" + mensagemDe(t);
        }
    }

    // ---------- datas (fuso do celular) ----------

    private static String fmtDia(Calendar c) {
        return String.format(java.util.Locale.US, "%04d-%02d-%02d",
                c.get(Calendar.YEAR), c.get(Calendar.MONTH) + 1, c.get(Calendar.DAY_OF_MONTH));
    }

    private static String fmtDiaBr(Calendar c) {
        return String.format(java.util.Locale.US, "%02d/%02d/%04d",
                c.get(Calendar.DAY_OF_MONTH), c.get(Calendar.MONTH) + 1, c.get(Calendar.YEAR));
    }

    /** "AAAA-MM-DD" -> Calendar à meia-noite (hoje se não der para ler). */
    private static Calendar lerDia(String s) {
        Calendar c = Calendar.getInstance();
        try {
            String[] p = s.split("-");
            c.set(Integer.parseInt(p[0]), Integer.parseInt(p[1]) - 1, Integer.parseInt(p[2]));
        } catch (Throwable ignorar) { }
        c.set(Calendar.HOUR_OF_DAY, 0);
        c.set(Calendar.MINUTE, 0);
        c.set(Calendar.SECOND, 0);
        c.set(Calendar.MILLISECOND, 0);
        return c;
    }

    private static Calendar copia(Calendar c) {
        return (Calendar) c.clone();
    }

    private static Calendar maisDias(Calendar c, int n) {
        Calendar r = copia(c);
        r.add(Calendar.DAY_OF_MONTH, n);
        return r;
    }

    // ---------- calendário para o Placar ----------

    private static volatile String dataEscolhida = "";

    public static String escolherData(String atual) {
        MainActivity a = ativa;
        if (a == null) return "";
        a.runOnUiThread(() -> {
            Calendar c = lerDia(atual);
            android.app.DatePickerDialog d = new android.app.DatePickerDialog(a,
                    (v, ano, mes, dia) -> {
                        Calendar e = Calendar.getInstance();
                        e.set(ano, mes, dia);
                        dataEscolhida = fmtDia(e);
                    },
                    c.get(Calendar.YEAR), c.get(Calendar.MONTH), c.get(Calendar.DAY_OF_MONTH));
            d.setOnCancelListener(x -> dataEscolhida = "CANCELADO");
            d.show();
        });
        return "";
    }

    /** O dia escolhido ("AAAA-MM-DD"), "CANCELADO" ou "" (nada ainda). Só devolve uma vez. */
    public static String dataEscolhida() {
        String d = dataEscolhida;
        dataEscolhida = "";
        return d;
    }

    // ---------- formulário de evento ----------

    public static String eventoEditar(String evento, String jogos) {
        MainActivity a = ativa;
        if (a == null) return "";
        a.runOnUiThread(() -> a.dialogoEvento(evento, jogos));
        return "";
    }

    private android.widget.Spinner lista(String[] itens, int sel) {
        android.widget.Spinner sp = new android.widget.Spinner(this);
        android.widget.ArrayAdapter<String> ad = new android.widget.ArrayAdapter<>(this,
                android.R.layout.simple_spinner_dropdown_item, itens);
        sp.setAdapter(ad);
        if (sel >= 0 && sel < itens.length) sp.setSelection(sel);
        return sp;
    }

    private TextView rotulo(String t) {
        TextView v = new TextView(this);
        v.setText(t);
        v.setTextSize(13);
        v.setPadding(0, dp(12), 0, dp(2));
        return v;
    }

    private void dialogoEvento(String eventoJson, String jogosJson) {
        if (!criador) {
            Toast.makeText(this, "Só o criador do app pode criar eventos.", Toast.LENGTH_LONG).show();
            return;
        }
        JSONObject ev;
        try { ev = eventoJson == null || eventoJson.isEmpty() ? new JSONObject() : new JSONObject(eventoJson); }
        catch (Throwable t) { ev = new JSONObject(); }
        final String id = ev.optString("id");
        final long criadoMs = ev.optLong("criado_ms", 0);

        // jogos para escolher: [[chave, nome], ...]
        List<String> chaves = new ArrayList<>();
        List<String> nomes = new ArrayList<>();
        try {
            JSONArray js = new JSONArray(jogosJson);
            for (int i = 0; i < js.length(); i++) {
                JSONArray j = js.getJSONArray(i);
                chaves.add(j.getString(0));
                nomes.add(j.getString(1));
            }
        } catch (Throwable ignorar) { }
        if (!ev.optString("jogo_chave").isEmpty() && !chaves.contains(ev.optString("jogo_chave"))) {
            chaves.add(ev.optString("jogo_chave"));
            nomes.add(ev.optString("jogo_nome", ev.optString("jogo_chave")));
        }

        LinearLayout l = new LinearLayout(this);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setPadding(dp(22), dp(8), dp(22), dp(8));

        final EditText nome = campo("Nome do evento", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES);
        nome.setText(ev.optString("nome"));
        l.addView(nome);
        final EditText descricao = campo("Descrição", InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES);
        descricao.setSingleLine(false);
        descricao.setMinLines(2);
        descricao.setText(ev.optString("descricao"));
        l.addView(descricao);
        final EditText recompensas = campo("Recompensas (ex.: 1º lugar: ...)", InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES);
        recompensas.setSingleLine(false);
        recompensas.setMinLines(2);
        recompensas.setText(ev.optString("recompensas"));
        l.addView(recompensas);

        final String[] tipos = {"dia", "semana", "mes", "periodo", "tudo"};
        int tipoSel = java.util.Arrays.asList(tipos).indexOf(ev.optString("tipo", "semana"));
        l.addView(rotulo("Período"));
        final android.widget.Spinner tipo = lista(new String[]{
                "Um dia", "Uma semana (segunda a domingo)", "Um mês",
                "Período (escolher início e fim)", "Tempo total (até uma data ou sem fim)"}, tipoSel < 0 ? 1 : tipoSel);
        l.addView(tipo);

        final Calendar[] ini = {lerDia(ev.optString("inicio", fmtDia(Calendar.getInstance())))};
        final Calendar[] fim = {lerDia(ev.optString("fim", fmtDia(maisDias(Calendar.getInstance(), 6))))};
        final android.widget.Button bIni = new android.widget.Button(this);
        final android.widget.Button bFim = new android.widget.Button(this);
        final android.widget.CheckBox semFim = new android.widget.CheckBox(this);
        semFim.setText("Sem data para acabar");
        semFim.setChecked("tudo".equals(ev.optString("tipo")) && ev.optString("fim").isEmpty());
        l.addView(bIni);
        l.addView(bFim);
        l.addView(semFim);

        l.addView(rotulo("O que conta"));
        final android.widget.Spinner oQue = lista(new String[]{"Tempo total (todos os jogos)", "Um jogo"},
                ev.optString("jogo_chave").isEmpty() ? 0 : 1);
        l.addView(oQue);
        final android.widget.Spinner jogo = lista(nomes.isEmpty() ? new String[]{"(nenhum jogo ainda)"}
                : nomes.toArray(new String[0]), Math.max(0, chaves.indexOf(ev.optString("jogo_chave"))));
        l.addView(jogo);

        final String[] plats = {"todas", "mobile", "steam", "psn"};
        int platSel = java.util.Arrays.asList(plats).indexOf(ev.optString("plataforma", "todas"));
        l.addView(rotulo("Plataforma"));
        final android.widget.Spinner plat = lista(new String[]{"Todas", "Mobile", "Steam", "PlayStation"},
                platSel < 0 ? 0 : platSel);
        l.addView(plat);

        l.addView(rotulo("Quantos vencedores (1 a 10)"));
        final EditText venc = campo("3", InputType.TYPE_CLASS_NUMBER);
        venc.setText(String.valueOf(ev.optInt("vencedores", 3)));
        l.addView(venc);

        // molduras de recompensa para cada colocação (pode ser mais de uma por lugar)
        final String[] molduraIds = {"ruby", "diamante", "bronze"};
        final String[] molduraNomes = {"Ruby", "Diamante", "Bronze"};
        JSONObject premiosAntes = ev.optJSONObject("premios");
        boolean novo = id.isEmpty();
        l.addView(rotulo("Molduras de recompensa por colocação"));
        @SuppressWarnings("unchecked")
        final java.util.LinkedHashSet<String>[] escolhas = new java.util.LinkedHashSet[10];
        final android.widget.Button[] botoesPremio = new android.widget.Button[10];
        final android.view.View[] linhasPremio = new android.view.View[10];
        for (int i = 0; i < 10; i++) {
            java.util.LinkedHashSet<String> e0 = new java.util.LinkedHashSet<>();
            if (premiosAntes != null) {
                Object v = premiosAntes.opt(String.valueOf(i + 1));
                if (v instanceof JSONArray) {
                    JSONArray a = (JSONArray) v;
                    for (int k = 0; k < a.length(); k++) if (!a.optString(k).isEmpty()) e0.add(a.optString(k));
                } else if (v instanceof String && !((String) v).isEmpty()) {
                    e0.add((String) v);
                }
            } else if (novo && i < 3) {
                e0.add(molduraIds[i]);
            }
            escolhas[i] = e0;
            final int lugar = i;
            android.widget.Button b = new android.widget.Button(this);
            b.setAllCaps(false);
            botoesPremio[i] = b;
            b.setOnClickListener(v -> {
                boolean[] marcadas = new boolean[molduraIds.length];
                for (int k = 0; k < molduraIds.length; k++) marcadas[k] = escolhas[lugar].contains(molduraIds[k]);
                new AlertDialog.Builder(this, temaDialogo())
                        .setTitle((lugar + 1) + "º lugar ganha")
                        .setMultiChoiceItems(molduraNomes, marcadas, (dd, k, marcada) -> {
                            if (marcada) escolhas[lugar].add(molduraIds[k]);
                            else escolhas[lugar].remove(molduraIds[k]);
                        })
                        .setPositiveButton("Pronto", (dd, w) -> textoPremio(b, lugar, escolhas[lugar], molduraIds, molduraNomes))
                        .setOnDismissListener(dd -> textoPremio(b, lugar, escolhas[lugar], molduraIds, molduraNomes))
                        .show();
            });
            textoPremio(b, i, escolhas[i], molduraIds, molduraNomes);
            linhasPremio[i] = b;
            l.addView(b);
        }
        final Runnable mostrarPremios = () -> {
            int n = 3;
            try { n = Math.max(1, Math.min(10, Integer.parseInt(venc.getText().toString().trim()))); }
            catch (Throwable ignorar) { }
            for (int i = 0; i < 10; i++) {
                linhasPremio[i].setVisibility(i < n ? android.view.View.VISIBLE : android.view.View.GONE);
            }
        };
        venc.addTextChangedListener(new android.text.TextWatcher() {
            @Override public void beforeTextChanged(CharSequence c, int a1, int a2, int a3) { }
            @Override public void onTextChanged(CharSequence c, int a1, int a2, int a3) { }
            @Override public void afterTextChanged(android.text.Editable e) { mostrarPremios.run(); }
        });
        mostrarPremios.run();

        // mostra só o que faz sentido para o tipo escolhido
        final Runnable atualizar = () -> {
            String t = tipos[tipo.getSelectedItemPosition()];
            boolean temFim = t.equals("periodo") || t.equals("tudo");
            String rotIni;
            switch (t) {
                case "dia": rotIni = "Dia: "; break;
                case "semana": rotIni = "Qualquer dia da semana: "; break;
                case "mes": rotIni = "Qualquer dia do mês: "; break;
                case "tudo": rotIni = "Aparece a partir de: "; break;
                default: rotIni = "Começa em: ";
            }
            bIni.setText(rotIni + fmtDiaBr(ini[0]));
            bFim.setText("Termina em: " + fmtDiaBr(fim[0]) + " (inclusive)");
            bFim.setVisibility(temFim && !(t.equals("tudo") && semFim.isChecked())
                    ? android.view.View.VISIBLE : android.view.View.GONE);
            semFim.setVisibility(t.equals("tudo") ? android.view.View.VISIBLE : android.view.View.GONE);
            jogo.setVisibility(oQue.getSelectedItemPosition() == 1
                    ? android.view.View.VISIBLE : android.view.View.GONE);
        };
        android.widget.AdapterView.OnItemSelectedListener mudou = new android.widget.AdapterView.OnItemSelectedListener() {
            @Override public void onItemSelected(android.widget.AdapterView<?> p, android.view.View v, int pos, long id2) { atualizar.run(); }
            @Override public void onNothingSelected(android.widget.AdapterView<?> p) { }
        };
        tipo.setOnItemSelectedListener(mudou);
        oQue.setOnItemSelectedListener(mudou);
        semFim.setOnCheckedChangeListener((b, v) -> atualizar.run());
        bIni.setOnClickListener(v -> new android.app.DatePickerDialog(this, (dp, a, m, d) -> {
            ini[0].set(a, m, d);
            atualizar.run();
        }, ini[0].get(Calendar.YEAR), ini[0].get(Calendar.MONTH), ini[0].get(Calendar.DAY_OF_MONTH)).show());
        bFim.setOnClickListener(v -> new android.app.DatePickerDialog(this, (dp, a, m, d) -> {
            fim[0].set(a, m, d);
            atualizar.run();
        }, fim[0].get(Calendar.YEAR), fim[0].get(Calendar.MONTH), fim[0].get(Calendar.DAY_OF_MONTH)).show());
        atualizar.run();

        android.widget.ScrollView rolagem = new android.widget.ScrollView(this);
        rolagem.addView(l);
        AlertDialog d = new AlertDialog.Builder(this, temaDialogo())
                .setTitle(id.isEmpty() ? "Novo evento" : "Editar evento")
                .setView(rolagem)
                .setPositiveButton("Salvar", null)
                .setNegativeButton("Cancelar", null)
                .create();
        d.setOnShowListener(x -> d.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener(v -> {
            String n = nome.getText().toString().trim();
            if (n.isEmpty()) { nome.setError("Dê um nome ao evento"); return; }
            String t = tipos[tipo.getSelectedItemPosition()];
            Calendar a, b;
            switch (t) {
                case "dia": a = copia(ini[0]); b = copia(ini[0]); break;
                case "semana": {
                    int dow = ini[0].get(Calendar.DAY_OF_WEEK); // domingo = 1, segunda = 2
                    a = maisDias(ini[0], -((dow + 5) % 7));
                    b = maisDias(a, 6);
                    break;
                }
                case "mes": {
                    a = copia(ini[0]);
                    a.set(Calendar.DAY_OF_MONTH, 1);
                    b = copia(a);
                    b.set(Calendar.DAY_OF_MONTH, a.getActualMaximum(Calendar.DAY_OF_MONTH));
                    break;
                }
                case "tudo": a = copia(ini[0]); b = semFim.isChecked() ? null : copia(fim[0]); break;
                default: a = copia(ini[0]); b = copia(fim[0]);
            }
            if (b != null && b.before(a)) { Toast.makeText(this, "O fim tem que ser depois do início.", Toast.LENGTH_LONG).show(); return; }
            int vencedores = 3;
            try { vencedores = Math.max(1, Math.min(10, Integer.parseInt(venc.getText().toString().trim()))); }
            catch (Throwable ignorar) { }
            boolean umJogo = oQue.getSelectedItemPosition() == 1 && !chaves.isEmpty();
            int j = Math.min(jogo.getSelectedItemPosition(), Math.max(0, chaves.size() - 1));

            long agora = System.currentTimeMillis();
            long inicioMs = lerDia(fmtDia(a)).getTimeInMillis();
            long fimMs = b == null ? 0 : lerDia(fmtDia(maisDias(b, 1))).getTimeInMillis();
            Map<String, Object> doc = new HashMap<>();
            doc.put("nome", n);
            doc.put("descricao", descricao.getText().toString().trim());
            doc.put("recompensas", recompensas.getText().toString().trim());
            doc.put("tipo", t);
            doc.put("inicio", fmtDia(a));
            doc.put("fim", b == null ? "" : fmtDia(b));
            doc.put("inicio_ms", inicioMs);
            doc.put("fim_ms", fimMs);
            doc.put("plataforma", plats[plat.getSelectedItemPosition()]);
            doc.put("jogo_chave", umJogo ? chaves.get(j) : "");
            doc.put("jogo_nome", umJogo ? nomes.get(j) : "");
            doc.put("vencedores", vencedores);
            Map<String, Object> mapaPremios = new HashMap<>();
            for (int i = 0; i < vencedores; i++) {
                if (!escolhas[i].isEmpty()) mapaPremios.put(String.valueOf(i + 1), new ArrayList<>(escolhas[i]));
            }
            doc.put("premios", mapaPremios);
            // o servidor entrega as molduras quando o evento acabar (de novo, se você editar)
            doc.put("premiado", false);
            doc.put("criado_ms", criadoMs > 0 ? criadoMs : agora);
            doc.put("autor", criadorUid);
            // avisos: só avisa o que ainda vai acontecer
            doc.put("avisado_inicio", inicioMs <= agora);
            doc.put("avisado_fim", fimMs == 0 || fimMs <= agora);

            v.setEnabled(false);
            FirebaseFirestore db = FirebaseFirestore.getInstance();
            com.google.android.gms.tasks.OnSuccessListener<Object> ok = r -> {
                eventosVersao++;
                d.dismiss();
                Toast.makeText(this, "Evento salvo", Toast.LENGTH_SHORT).show();
            };
            com.google.android.gms.tasks.OnFailureListener falha = e -> {
                v.setEnabled(true);
                String m = mensagemDe(e);
                if (m.contains("PERMISSION_DENIED") || m.toLowerCase().contains("permission")) {
                    m = "Só o criador do app pode salvar eventos (confira o abysshub_admins e as regras).";
                }
                Toast.makeText(this, m, Toast.LENGTH_LONG).show();
            };
            if (id.isEmpty()) {
                db.collection(EVENTOS).add(doc).addOnSuccessListener(ok).addOnFailureListener(falha);
            } else {
                db.collection(EVENTOS).document(id).set(doc).addOnSuccessListener(ok).addOnFailureListener(falha);
            }
        }));
        d.show();
    }

    /** "1º lugar: Ruby + Diamante" no botão do formulário. */
    private static void textoPremio(android.widget.Button b, int lugar, java.util.Set<String> escolhas,
                                    String[] ids, String[] nomes) {
        StringBuilder t = new StringBuilder();
        for (int k = 0; k < ids.length; k++) {
            if (escolhas.contains(ids[k])) t.append(t.length() == 0 ? "" : " + ").append(nomes[k]);
        }
        b.setText((lugar + 1) + "º lugar: " + (t.length() == 0 ? "nenhuma moldura" : t));
    }

    /** Troca só as molduras de recompensa de um evento. Devolve "" se deu certo. */
    public static String eventoPremios(String id, String json) {
        try {
            if (!criador) return "Só o criador do app pode mudar as recompensas.";
            Map<String, Object> m = new HashMap<>();
            m.put("premios", mapa(new JSONObject(json)));
            // o servidor entrega de novo (com as molduras novas) se o evento já acabou
            m.put("premiado", false);
            Tasks.await(FirebaseFirestore.getInstance().collection(EVENTOS).document(id).update(m),
                    20, TimeUnit.SECONDS);
            eventosVersao++;
            return "";
        } catch (Throwable t) {
            String msg = mensagemDe(t);
            if (msg.contains("PERMISSION_DENIED")) return "Só o criador do app pode mudar as recompensas.";
            return msg;
        }
    }

    public static String eventoApagar(String id) {
        MainActivity a = ativa;
        if (a == null || id == null || id.isEmpty()) return "";
        a.runOnUiThread(() -> new AlertDialog.Builder(a, a.temaDialogo())
                .setTitle("Apagar evento?")
                .setMessage("O evento some para todo mundo. Não dá para desfazer.")
                .setPositiveButton("Apagar", (dd, w) -> FirebaseFirestore.getInstance().collection(EVENTOS).document(id)
                        .delete()
                        .addOnSuccessListener(r -> {
                            eventosVersao++;
                            Toast.makeText(a, "Evento apagado", Toast.LENGTH_SHORT).show();
                        })
                        .addOnFailureListener(e -> Toast.makeText(a, mensagemDe(e), Toast.LENGTH_LONG).show()))
                .setNegativeButton("Cancelar", null)
                .show());
        return "";
    }

    // ---------- avisos (notificações) dos eventos ----------

    private void prepararAvisos() {
        try {
            android.app.NotificationChannel canal = new android.app.NotificationChannel(
                    "eventos", "Eventos", android.app.NotificationManager.IMPORTANCE_DEFAULT);
            canal.setDescription("Quando um evento começa ou termina");
            getSystemService(android.app.NotificationManager.class).createNotificationChannel(canal);
            if (firebasePronto(this)) FirebaseMessaging.getInstance().subscribeToTopic("abysshub_eventos");
            if (Build.VERSION.SDK_INT >= 33
                    && checkSelfPermission("android.permission.POST_NOTIFICATIONS") != PackageManager.PERMISSION_GRANTED) {
                SharedPreferences p = getSharedPreferences("abysshub_avisos", MODE_PRIVATE);
                if (!p.getBoolean("pediu", false)) {
                    p.edit().putBoolean("pediu", true).apply();
                    requestPermissions(new String[]{"android.permission.POST_NOTIFICATIONS"}, 77);
                }
            }
        } catch (Throwable t) {
            Log.w(TAG, "avisos", t);
        }
    }

    // =================================================================
    // PlayStation — quem lê a PSN é o servidor do AbyssHub (Cloud Functions), com uma conta "robô".
    // A pessoa só prova que a conta é dela colocando um código no "Sobre" do perfil.
    // =================================================================

    private static final String REGIAO_FUNCOES = "southamerica-east1";

    /** "" = ver as preferências; "conectando"; "erro" (com psnMsg). */
    private static volatile String psnEstado = "";
    private static volatile String psnMsg = "";

    private static SharedPreferences prefsPsn() {
        Context c = contexto;
        return c == null ? null : c.getSharedPreferences("abysshub_psn", MODE_PRIVATE);
    }

    private static com.google.android.gms.tasks.Task<HttpsCallableResult> chamar(String nome, Map<String, Object> dados) {
        return FirebaseFunctions.getInstance(REGIAO_FUNCOES).getHttpsCallable(nome)
                .call(dados == null ? new HashMap<String, Object>() : dados);
    }

    /** Mensagem do servidor (em português) ou, se não tiver, a do erro. */
    private static String mensagemFuncao(Throwable t) {
        for (Throwable c = t; c != null; c = c.getCause() == c ? null : c.getCause()) {
            if (c instanceof FirebaseFunctionsException) {
                FirebaseFunctionsException f = (FirebaseFunctionsException) c;
                if (f.getCode() == FirebaseFunctionsException.Code.NOT_FOUND && "NOT_FOUND".equals(f.getMessage())) {
                    return "O servidor da PlayStation ainda não foi publicado.";
                }
                if (f.getCode() == FirebaseFunctionsException.Code.UNAVAILABLE
                        || f.getCode() == FirebaseFunctionsException.Code.DEADLINE_EXCEEDED) {
                    String m = f.getMessage();
                    return m == null || m.equals("UNAVAILABLE") || m.equals("DEADLINE_EXCEEDED")
                            ? "Sem conexão com o servidor. Tente de novo." : m;
                }
                if (f.getMessage() != null) return f.getMessage();
            }
        }
        return mensagemDe(t);
    }

    /** {"estado": "desligado"|"conectando"|"pendente"|"ok"|"erro", "msg", "nome"} */
    public static String psnEstado() {
        try {
            JSONObject o = new JSONObject();
            SharedPreferences p = prefsPsn();
            boolean verificado = p != null && p.getBoolean("verificado", false);
            boolean temCodigo = p != null && !p.getString("codigo", "").isEmpty();
            String e = psnEstado;
            if (e.equals("conectando")) o.put("estado", "conectando");
            else if (verificado) o.put("estado", "ok");
            else if (temCodigo) o.put("estado", "pendente");
            else if (e.equals("erro")) o.put("estado", "erro");
            else o.put("estado", "desligado");
            o.put("msg", verificado ? "" : psnMsg);
            o.put("nome", p == null ? "" : p.getString("nick", ""));
            return o.toString();
        } catch (Throwable t) {
            return "{\"estado\":\"desligado\"}";
        }
    }

    public static void psnConectar() {
        MainActivity a = ativa;
        if (a == null) return;
        a.runOnUiThread(() -> {
            SharedPreferences p = prefsPsn();
            if (p != null && !p.getString("codigo", "").isEmpty() && !p.getBoolean("verificado", false)) {
                a.dialogoPsnCodigo();
            } else {
                a.dialogoPsnNick();
            }
        });
    }

    public static void psnDesconectar() {
        SharedPreferences p = prefsPsn();
        if (p != null) p.edit().clear().apply();
        psnEstado = "";
        psnMsg = "";
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() != null) chamar("psnDesconectar", null);
        } catch (Throwable ignorar) { }
    }

    private void dialogoPsnNick() {
        if (!firebasePronto(this)) {
            Toast.makeText(this, "A PlayStation não está disponível nesta versão do app.", Toast.LENGTH_LONG).show();
            return;
        }
        if (FirebaseAuth.getInstance().getCurrentUser() == null) {
            Toast.makeText(this, "Entre na sua conta do AbyssHub primeiro.", Toast.LENGTH_LONG).show();
            return;
        }
        LinearLayout l = new LinearLayout(this);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setPadding(dp(22), dp(8), dp(22), 0);
        TextView t = new TextView(this);
        t.setText("Digite o seu ID online da PSN (o seu nick, igualzinho, com maiúsculas e minúsculas).\n\n"
                + "Depois o app te dá um código para colocar no \"Sobre\" do seu perfil. É só para provar que a "
                + "conta é sua: você não precisa dar a senha.");
        t.setTextSize(13);
        t.setPadding(0, 0, 0, dp(8));
        l.addView(t);
        final EditText nick = campo("ID online da PSN",
                InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD);
        SharedPreferences p = prefsPsn();
        if (p != null) nick.setText(p.getString("nick", ""));
        l.addView(nick);

        AlertDialog d = new AlertDialog.Builder(this, temaDialogo())
                .setTitle("Conectar PlayStation")
                .setView(l)
                .setPositiveButton("Gerar código", null)
                .setNegativeButton("Cancelar", null)
                .create();
        d.setOnShowListener(x -> d.getButton(DialogInterface.BUTTON_POSITIVE).setOnClickListener(v -> {
            String n = nick.getText().toString().trim();
            if (!n.matches("[A-Za-z0-9_-]{3,16}")) {
                nick.setError("O ID online tem de 3 a 16 letras, números, - ou _");
                return;
            }
            d.dismiss();
            psnEstado = "conectando";
            psnMsg = "";
            Map<String, Object> dados = new HashMap<>();
            dados.put("nick", n);
            try {
                chamar("psnIniciar", dados)
                        .addOnSuccessListener(r -> {
                            Object o = r.getData();
                            Map<?, ?> m = o instanceof Map ? (Map<?, ?>) o : new HashMap<>();
                            SharedPreferences pp = prefsPsn();
                            if (pp != null) {
                                pp.edit().clear()
                                        .putString("nick", textoDe(m.get("nick")).isEmpty() ? n : textoDe(m.get("nick")))
                                        .putString("codigo", textoDe(m.get("codigo")))
                                        .putBoolean("verificado", false)
                                        .apply();
                            }
                            psnEstado = "";
                            MainActivity a = ativa;
                            if (a != null) a.dialogoPsnCodigo();
                        })
                        .addOnFailureListener(e -> {
                            psnMsg = mensagemFuncao(e);
                            psnEstado = "erro";
                            avisar(psnMsg);
                        });
            } catch (Throwable e) {
                psnMsg = mensagemFuncao(e);
                psnEstado = "erro";
                avisar(psnMsg);
            }
        }));
        d.show();
    }

    private void dialogoPsnCodigo() {
        SharedPreferences p = prefsPsn();
        if (p == null) return;
        final String codigo = p.getString("codigo", "");
        if (codigo.isEmpty()) { dialogoPsnNick(); return; }

        LinearLayout l = new LinearLayout(this);
        l.setOrientation(LinearLayout.VERTICAL);
        l.setPadding(dp(22), dp(8), dp(22), 0);
        TextView passos = new TextView(this);
        passos.setText("1. Copie o código abaixo.\n"
                + "2. No app PlayStation, no console ou em playstation.com: abra o seu perfil, toque em "
                + "Editar perfil e cole o código no \"Sobre\". Salve.\n"
                + "3. Em Configurações > Privacidade da PSN, deixe tudo como \"Qualquer pessoa\" "
                + "(principalmente quem pode ver os seus jogos).\n"
                + "4. Volte aqui e toque em Verificar.\n\n"
                + "Depois de verificar, você já pode apagar o código do \"Sobre\".");
        passos.setTextSize(13);
        l.addView(passos);
        TextView cod = new TextView(this);
        cod.setText(codigo);
        cod.setTextSize(26);
        cod.setTextIsSelectable(true);
        cod.setGravity(Gravity.CENTER);
        cod.setTypeface(android.graphics.Typeface.MONOSPACE, android.graphics.Typeface.BOLD);
        cod.setPadding(0, dp(14), 0, dp(10));
        l.addView(cod);
        final TextView aviso = new TextView(this);
        aviso.setTextSize(13);
        aviso.setTextColor(0xFFE5534B);
        aviso.setText(psnMsg);
        l.addView(aviso);

        AlertDialog d = new AlertDialog.Builder(this, temaDialogo())
                .setTitle("Código da PlayStation")
                .setView(l)
                .setPositiveButton("Verificar", null)
                .setNeutralButton("Copiar código", null)
                .setNegativeButton("Depois", null)
                .create();
        d.setOnShowListener(x -> {
            d.getButton(DialogInterface.BUTTON_NEUTRAL).setOnClickListener(v -> {
                try {
                    android.content.ClipboardManager cm =
                            (android.content.ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
                    cm.setPrimaryClip(android.content.ClipData.newPlainText("Código AbyssHub", codigo));
                    Toast.makeText(this, "Código copiado", Toast.LENGTH_SHORT).show();
                } catch (Throwable ignorar) { }
            });
            android.widget.Button verificar = d.getButton(DialogInterface.BUTTON_POSITIVE);
            verificar.setOnClickListener(v -> {
                verificar.setEnabled(false);
                aviso.setTextColor(cod.getCurrentTextColor());
                aviso.setText("Verificando...");
                try {
                    chamar("psnVerificar", null)
                            .addOnSuccessListener(r -> {
                                SharedPreferences pp = prefsPsn();
                                if (pp != null) pp.edit().putBoolean("verificado", true).apply();
                                psnMsg = "";
                                psnEstado = "";
                                d.dismiss();
                                Toast.makeText(this, "PlayStation conectada! Já pode tirar o código do \"Sobre\".",
                                        Toast.LENGTH_LONG).show();
                            })
                            .addOnFailureListener(e -> {
                                psnMsg = mensagemFuncao(e);
                                verificar.setEnabled(true);
                                aviso.setTextColor(0xFFE5534B);
                                aviso.setText(psnMsg);
                            });
                } catch (Throwable e) {
                    psnMsg = mensagemFuncao(e);
                    verificar.setEnabled(true);
                    aviso.setTextColor(0xFFE5534B);
                    aviso.setText(psnMsg);
                }
            });
        });
        d.show();
    }

    /** Jogos com tempo: {"id", "jogos": [{appid, nome, min, ultima}]}, "DESLIGADO" ou "ERRO:msg". Bloqueia. */
    public static String psnJogos() {
        SharedPreferences p = prefsPsn();
        if (p == null) return "ERRO:O app ainda está abrindo";
        if (!p.getBoolean("verificado", false)) return "DESLIGADO";
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) {
                return "ERRO:Entre na sua conta do AbyssHub para atualizar a PlayStation.";
            }
            HttpsCallableResult r = Tasks.await(chamar("psnJogos", null), 70, TimeUnit.SECONDS);
            Object o = r.getData();
            if (!(o instanceof Map)) return "ERRO:Resposta estranha do servidor.";
            JSONObject j = new JSONObject((Map<?, ?>) o);
            // guarda as imagens dos jogos para os ícones
            JSONObject icones = new JSONObject(p.getString("icones", "{}"));
            JSONArray jogos = j.optJSONArray("jogos");
            if (jogos != null) {
                for (int i = 0; i < jogos.length(); i++) {
                    JSONObject g = jogos.optJSONObject(i);
                    if (g == null) continue;
                    String img = g.optString("imagem");
                    if (!img.isEmpty()) icones.put(g.optString("appid"), img);
                }
            }
            p.edit().putString("icones", icones.toString()).apply();
            return j.toString();
        } catch (Throwable t) {
            return "ERRO:PlayStation: " + mensagemFuncao(t);
        }
    }

    /** Capa do jogo da PSN ("psn:{titleId}"), da imagem que o servidor mandou. */
    private static Bitmap bitmapPsn(String pacote) {
        SharedPreferences p = prefsPsn();
        if (p == null) return null;
        try {
            String url = new JSONObject(p.getString("icones", "{}")).optString(pacote.substring("psn:".length()));
            return url.startsWith("https://") ? baixarBitmap(url) : null;
        } catch (Throwable t) {
            return null;
        }
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
            if (!perfilUid.equals(u.getUid())) carregarPerfil(u.getUid());
            String nome = nomeEfetivo(u);
            String foto = fotoEfetiva(u);
            long agora = System.currentTimeMillis();
            String dia = p.optString("dia");

            WriteBatch b = db.batch();
            Map<String, Object> pessoa = new HashMap<>();
            pessoa.put("uid", uid);
            pessoa.put("nome", nome);
            pessoa.put("foto", foto);
            pessoa.put("moldura", molduraEfetiva(u));
            pessoa.put("dia", dia);
            pessoa.put("atualizado_ms", agora);
            for (String per : new String[]{"hoje", "semana", "tudo"}) {
                pessoa.put(per + "_ms", p.optLong(per + "_ms"));
                JSONObject top = p.optJSONObject("top_" + per);
                pessoa.put(per + "_top_pacote", top != null ? top.optString("pacote") : "");
                pessoa.put(per + "_top_nome", top != null ? top.optString("nome") : "");
            }
            // tempo por período e plataforma ("semana_steam", "hoje_2026-11-18_todas"...) e o jogo top de cada um
            pessoa.put("r", mapa(p.optJSONObject("r")));
            pessoa.put("top", mapa(p.optJSONObject("top")));
            b.set(db.collection(PESSOAS).document(uid), pessoa);

            JSONArray jogos = p.optJSONArray("jogos");
            Set<String> meus = new HashSet<>();
            if (jogos != null) {
                for (int i = 0; i < jogos.length(); i++) {
                    JSONObject j = jogos.getJSONObject(i);
                    String pacote = j.optString("pacote");
                    if (pacote.isEmpty()) continue;
                    // a chave é o nome do jogo: junta o mesmo jogo de lojas diferentes
                    String chave = j.optString("chave", pacote);
                    Map<String, Object> par = new HashMap<>();
                    par.put("uid", uid);
                    par.put("nome", nome);
                    par.put("foto", foto);
                    par.put("moldura", molduraEfetiva(u));
                    par.put("pacote", pacote);
                    par.put("chave", chave);
                    par.put("jogo", j.optString("nome"));
                    par.put("dia", dia);
                    par.put("atualizado_ms", agora);
                    par.put("hoje_ms", j.optLong("hoje_ms"));
                    par.put("semana_ms", j.optLong("semana_ms"));
                    par.put("tudo_ms", j.optLong("tudo_ms"));
                    Map<String, Object> r = mapa(j.optJSONObject("r"));
                    par.put("r", r);
                    // o mesmo tempo dentro de "j.{chave}" — assim o modo "Por jogo" não precisa de índice
                    Map<String, Object> porJogo = new HashMap<>();
                    porJogo.put(chave, r);
                    par.put("j", porJogo);
                    par.put("plats", lista(j.optJSONArray("plats")));
                    b.set(db.collection(PARES).document(uid + "__" + chave), par);
                    meus.add(uid + "__" + chave);
                }
            }
            Tasks.await(b.commit(), 20, TimeUnit.SECONDS);
            limparParesAntigos(db, uid, meus);
            if (jogos != null) enviarIcones(db, jogos);
            return "";
        } catch (Throwable t) {
            return mensagemDe(t);
        }
    }

    private static volatile String limpouPara = "";

    /**
     * Uma vez por vez que o app abre: apaga os documentos antigos desta pessoa que não existem mais
     * (jogos desinstalados ou do formato antigo, antes de juntar jogos repetidos).
     */
    private static void limparParesAntigos(FirebaseFirestore db, String uid, Set<String> meus) {
        if (uid.equals(limpouPara)) return;
        try {
            QuerySnapshot s = Tasks.await(db.collection(PARES).whereEqualTo("uid", uid).get(), 20, TimeUnit.SECONDS);
            WriteBatch b = db.batch();
            int n = 0;
            for (DocumentSnapshot d : s.getDocuments()) {
                if (!meus.contains(d.getId())) {
                    b.delete(d.getReference());
                    n++;
                }
            }
            if (n > 0) Tasks.await(b.commit(), 20, TimeUnit.SECONDS);
            limpouPara = uid;
        } catch (Throwable t) {
            Log.w(TAG, "limpar", t);
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

    private static final long MES_MS = 30L * 24 * 60 * 60 * 1000;

    /** Mesmo nome de campo de ranking::campo no Rust. */
    private static String campoRanking(String periodo, String plataforma, String dia) {
        return (periodo.equals("hoje") ? "hoje_" + dia : periodo) + "_" + plataforma;
    }

    /**
     * modo: "pessoas" | "pares" | "jogo"; periodo: "hoje" | "semana" | "mes" | "tudo";
     * plataforma: "todas" | "mobile" | "steam"; chave: a chave do jogo (modo "jogo").
     * Devolve um JSON (lista) ou "ERRO:mensagem".
     */
    public static String buscar(String modo, String periodo, String plataforma, String chave, String dia) {
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) return "ERRO:Sem conta";
            FirebaseFirestore db = FirebaseFirestore.getInstance();
            String colecao = modo.equals("pessoas") ? PESSOAS : PARES;
            String campo = campoRanking(periodo, plataforma, dia);
            long agora = System.currentTimeMillis();
            // documento -> linha do ranking (assim ninguém aparece duas vezes)
            Map<String, JSONObject> linhas = new HashMap<>();

            // 1) formato novo (v2.1+): só aparece quem tem esse campo. Ordenar por um campo dentro de um
            //    mapa usa o índice automático do Firestore (não precisa criar índice composto).
            FieldPath fp = modo.equals("jogo") ? FieldPath.of("j", chave, campo) : FieldPath.of("r", campo);
            QuerySnapshot snap = Tasks.await(db.collection(colecao)
                    .orderBy(fp, Query.Direction.DESCENDING).limit(60).get(), 20, TimeUnit.SECONDS);
            for (DocumentSnapshot d : snap.getDocuments()) {
                Object v = d.get(fp);
                long ms = v instanceof Number ? ((Number) v).longValue() : 0L;
                JSONObject o = linha(d, modo, periodo, ms, agora);
                if (o == null) continue;
                if (modo.equals("pessoas")) {
                    Object t = d.get(FieldPath.of("top", campo));
                    if (t instanceof Map) {
                        Map<?, ?> top = (Map<?, ?>) t;
                        o.put("pacote", textoDe(top.get("pacote")));
                        o.put("jogo", textoDe(top.get("nome")));
                        o.put("chave", textoDe(top.get("chave")));
                        o.put("plats", listaJson(top.get("plats")));
                    }
                } else {
                    o.put("plats", listaJson(d.get("plats")));
                }
                linhas.put(d.getId(), o);
            }

            // 2) formato antigo (quem ainda está na v2.0 ou antes): só tem tempo do celular e não tem "Mês"
            boolean temAntigo = !periodo.equals("mes") && (plataforma.equals("todas") || plataforma.equals("mobile"));
            if (temAntigo) {
                try {
                    String campoAntigo = periodo + "_ms";
                    Query q = db.collection(colecao);
                    if (modo.equals("jogo")) q = q.whereEqualTo("chave", chave);
                    if (periodo.equals("hoje")) q = q.whereEqualTo("dia", dia);
                    q = q.orderBy(campoAntigo, Query.Direction.DESCENDING).limit(60);
                    QuerySnapshot antigos = Tasks.await(q.get(), 20, TimeUnit.SECONDS);
                    for (DocumentSnapshot d : antigos.getDocuments()) {
                        if (d.contains("r")) continue; // versão nova: já veio na busca 1
                        if (linhas.containsKey(d.getId())) continue;
                        JSONObject o = linha(d, modo, periodo, numero(d, campoAntigo), agora);
                        if (o == null) continue;
                        if (modo.equals("pessoas")) {
                            o.put("pacote", texto(d, periodo + "_top_pacote"));
                            o.put("jogo", texto(d, periodo + "_top_nome"));
                        }
                        JSONArray plats = new JSONArray();
                        plats.put("mobile");
                        o.put("plats", plats);
                        linhas.put(d.getId(), o);
                    }
                } catch (Throwable t) {
                    Log.w(TAG, "ranking antigo", t); // sem o formato antigo, mostra só o novo
                }
            }

            List<JSONObject> lista = new ArrayList<>(linhas.values());
            java.util.Collections.sort(lista, (a, b) -> Long.compare(b.optLong("ms"), a.optLong("ms")));
            JSONArray saida = new JSONArray();
            for (int i = 0; i < lista.size() && i < 50; i++) saida.put(lista.get(i));
            return saida.toString();
        } catch (Throwable t) {
            return "ERRO:" + mensagemDe(t);
        }
    }

    /** Parte comum de uma linha do ranking. null = não entra (sem tempo ou sem abrir o app há muito tempo). */
    private static JSONObject linha(DocumentSnapshot d, String modo, String periodo, long ms, long agora) throws Exception {
        long atualizado = numero(d, "atualizado_ms");
        if (ms <= 0) return null;
        // quem não abre o app há mais de 7 (ou 30) dias não conta na semana (ou no mês)
        if (periodo.equals("semana") && agora - atualizado > SEMANA_MS) return null;
        if (periodo.equals("mes") && agora - atualizado > MES_MS) return null;
        JSONObject o = new JSONObject();
        o.put("uid", texto(d, "uid"));
        o.put("nome", texto(d, "nome"));
        o.put("foto", texto(d, "foto"));
        o.put("moldura", texto(d, "moldura"));
        o.put("ms", ms);
        o.put("atualizado_ms", atualizado);
        if (!modo.equals("pessoas")) {
            o.put("pacote", texto(d, "pacote"));
            o.put("jogo", texto(d, "jogo"));
            o.put("chave", texto(d, "chave"));
        }
        return o;
    }

    /** JSONObject -> Map (para gravar no Firestore). */
    private static Map<String, Object> mapa(JSONObject o) throws Exception {
        Map<String, Object> m = new HashMap<>();
        if (o == null) return m;
        Iterator<String> k = o.keys();
        while (k.hasNext()) {
            String n = k.next();
            Object v = o.get(n);
            if (v instanceof JSONObject) m.put(n, mapa((JSONObject) v));
            else if (v instanceof JSONArray) m.put(n, lista((JSONArray) v));
            else if (v != JSONObject.NULL) m.put(n, v);
        }
        return m;
    }

    private static List<Object> lista(JSONArray a) throws Exception {
        List<Object> l = new ArrayList<>();
        if (a == null) return l;
        for (int i = 0; i < a.length(); i++) {
            Object v = a.get(i);
            if (v instanceof JSONObject) l.add(mapa((JSONObject) v));
            else if (v instanceof JSONArray) l.add(lista((JSONArray) v));
            else if (v != JSONObject.NULL) l.add(v);
        }
        return l;
    }

    private static JSONArray listaJson(Object o) {
        JSONArray a = new JSONArray();
        if (o instanceof List) for (Object x : (List<?>) o) if (x instanceof String) a.put(x);
        return a;
    }

    private static String textoDe(Object o) {
        return o instanceof String ? (String) o : "";
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
            b.delete(db.collection(HISTORICO).document(u.getUid()));
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
        if (url.startsWith("fs:")) return fotoGuardada(url, tam);
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

    /** Foto escolhida da galeria (endereço "fs:{uid}:{versão}"), guardada no Firestore. */
    private static byte[] fotoGuardada(String url, int tam) {
        try {
            if (FirebaseAuth.getInstance().getCurrentUser() == null) return null;
            String[] partes = url.split(":");
            if (partes.length < 2) return null;
            DocumentSnapshot s = Tasks.await(
                    FirebaseFirestore.getInstance().collection(FOTOS).document(partes[1]).get(), 15, TimeUnit.SECONDS);
            Object o = s.get("jpg");
            if (!(o instanceof String)) return null;
            byte[] bytes = Base64.decode((String) o, Base64.DEFAULT);
            Bitmap b = BitmapFactory.decodeByteArray(bytes, 0, bytes.length);
            return b == null ? null : paraRgba(b, tam);
        } catch (Throwable t) {
            return null;
        }
    }

    private static Bitmap bitmapDoIcone(String pacote, int tam) {
        if (pacote.startsWith("steam:")) {
            Bitmap b = bitmapSteam(pacote);
            return b == null ? null : quadrado(b, tam);
        }
        if (pacote.startsWith("psn:")) {
            Bitmap b = bitmapPsn(pacote);
            return b == null ? null : quadrado(b, tam);
        }
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

    /** Recorta um quadrado no meio e redimensiona para tam x tam. */
    private static Bitmap quadrado(Bitmap origem, int tam) {
        Bitmap b = Bitmap.createBitmap(tam, tam, Bitmap.Config.ARGB_8888);
        Canvas c = new Canvas(b);
        int w = origem.getWidth(), h = origem.getHeight(), lado = Math.min(w, h);
        Rect src = new Rect((w - lado) / 2, (h - lado) / 2, (w - lado) / 2 + lado, (h - lado) / 2 + lado);
        c.drawBitmap(origem, src, new Rect(0, 0, tam, tam), new Paint(Paint.FILTER_BITMAP_FLAG | Paint.ANTI_ALIAS_FLAG));
        return b;
    }

    /** Recorta no meio, redimensiona e devolve os bytes RGBA. */
    private static byte[] paraRgba(Bitmap origem, int tam) {
        Bitmap b = quadrado(origem, tam);
        ByteBuffer buf = ByteBuffer.allocate(tam * tam * 4);
        b.copyPixelsToBuffer(buf);
        return buf.array();
    }
}
