package com.abysshub.app;

import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.Build;

import com.google.firebase.messaging.FirebaseMessagingService;
import com.google.firebase.messaging.RemoteMessage;

/**
 * Mostra os avisos de eventos quando o app está aberto (com o app fechado, o próprio Android mostra).
 * Os avisos são mandados pelo servidor (functions/index.js -> avisarEventos) para o tópico
 * "abysshub_eventos".
 */
public class AvisosService extends FirebaseMessagingService {

    @Override
    public void onMessageReceived(RemoteMessage m) {
        try {
            RemoteMessage.Notification n = m.getNotification();
            String titulo = n != null ? n.getTitle() : m.getData().get("titulo");
            String texto = n != null ? n.getBody() : m.getData().get("texto");
            if (titulo == null) return;
            if (Build.VERSION.SDK_INT >= 33
                    && checkSelfPermission("android.permission.POST_NOTIFICATIONS") != PackageManager.PERMISSION_GRANTED) {
                return;
            }
            Intent abrir = new Intent(this, MainActivity.class);
            abrir.setFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);
            PendingIntent pi = PendingIntent.getActivity(this, 0, abrir,
                    PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);
            Notification aviso = new Notification.Builder(this, "eventos")
                    .setSmallIcon(R.drawable.ic_aviso)
                    .setContentTitle(titulo)
                    .setContentText(texto == null ? "" : texto)
                    .setStyle(new Notification.BigTextStyle().bigText(texto == null ? "" : texto))
                    .setAutoCancel(true)
                    .setContentIntent(pi)
                    .build();
            NotificationManager nm = getSystemService(NotificationManager.class);
            if (nm != null) nm.notify((int) (System.currentTimeMillis() % Integer.MAX_VALUE), aviso);
        } catch (Throwable ignorar) {
        }
    }

    @Override
    public void onNewToken(String token) {
        // usamos só o tópico "abysshub_eventos"; não precisa guardar o token
    }
}
