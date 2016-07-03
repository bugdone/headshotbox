package hsbox.java;

import java.awt.*;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.io.File;
import java.net.URI;

public class SysTrayIcon {
    public static void openWebpage(URI uri) {
        Desktop desktop = Desktop.isDesktopSupported() ? Desktop.getDesktop() : null;
        if (desktop != null && desktop.isSupported(Desktop.Action.BROWSE)) {
            try {
                desktop.browse(uri);
            } catch (Exception e) {
                e.printStackTrace();
            }
        }
    }

    public SysTrayIcon(final URI uri, final File log) {
        if (SystemTray.isSupported()) {
            Image image = Toolkit.getDefaultToolkit().getImage("resources/public/img/hsbox.png");
            PopupMenu popup = new PopupMenu();

            MenuItem openItem = new MenuItem("Open");
            openItem.addActionListener(new ActionListener() {
                public void actionPerformed(ActionEvent e) {
                    openWebpage(uri);
                }
            });
            popup.add(openItem);
            popup.addSeparator();

            MenuItem openLogItem = new MenuItem("Open log file");
            openLogItem.addActionListener(new ActionListener() {
                public void actionPerformed(ActionEvent e) {
                    openWebpage(log.toURI());
                }
            });
            popup.add(openLogItem);
            popup.addSeparator();

            MenuItem quitItem = new MenuItem("Quit");
            quitItem.addActionListener(new ActionListener() {
                public void actionPerformed(ActionEvent e) {
                    System.exit(0);
                }
            });
            popup.add(quitItem);

            TrayIcon trayIcon = new TrayIcon(image, "HeadshotBox", popup);
            trayIcon.setImageAutoSize(true);
            try {
                SystemTray.getSystemTray().add(trayIcon);
            } catch (AWTException e) {
                System.err.println(e);
            }
        }
    }
}
