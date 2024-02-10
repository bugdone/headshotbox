package hsbox.java;

import javax.imageio.ImageIO;
import java.awt.*;
import java.awt.event.ActionEvent;
import java.awt.event.ActionListener;
import java.awt.image.BufferedImage;
import java.io.File;
import java.io.IOException;
import java.io.InputStream;
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
            try {
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

                InputStream is = ClassLoader.getSystemClassLoader().getResourceAsStream("newui/public/images/hsbox.png");
                BufferedImage image = ImageIO.read(is);
                TrayIcon trayIcon = new TrayIcon(image, "HeadshotBox", popup);
                trayIcon.setImageAutoSize(true);
                SystemTray.getSystemTray().add(trayIcon);
            } catch (IOException e) {
                System.err.println(e);
            } catch (AWTException e) {
                System.err.println(e);
            }
        }
    }
}
