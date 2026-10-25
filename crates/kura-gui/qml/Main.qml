// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami
import org.kura.app

Kirigami.ApplicationWindow {
    id: root

    title: "kura"
    width: Kirigami.Units.gridUnit * 50
    height: Kirigami.Units.gridUnit * 34
    minimumWidth: Kirigami.Units.gridUnit * 22

    Kura {
        id: kura
        Component.onCompleted: refresh()
    }

    globalDrawer: Kirigami.GlobalDrawer {
        title: "kura"
        titleIcon: "system-software-install"
        modal: !root.wideScreen
        isMenu: false
        actions: [
            Kirigami.Action {
                text: kura.tr("nav-overview")
                icon.name: "go-home"
                checked: true
            },
            Kirigami.Action {
                text: kura.tr("nav-history")
                icon.name: "view-history"
                enabled: false
            },
            Kirigami.Action {
                text: kura.tr("nav-settings")
                icon.name: "settings-configure"
                enabled: false
            }
        ]
    }

    pageStack.initialPage: Kirigami.ScrollablePage {
        title: kura.tr("nav-overview")

        actions: [
            Kirigami.Action {
                text: kura.tr("overview-refresh")
                icon.name: "view-refresh"
                onTriggered: kura.refresh()
            }
        ]

        ColumnLayout {
            spacing: Kirigami.Units.largeSpacing

            Kirigami.InlineMessage {
                Layout.fillWidth: true
                visible: true
                type: Kirigami.MessageType.Information
                text: kura.tr("prototype-note")
            }

            Kirigami.InlineMessage {
                Layout.fillWidth: true
                visible: kura.error.length > 0
                type: Kirigami.MessageType.Error
                text: kura.error
            }

            Controls.Label {
                text: kura.tr("overview-subtitle")
                color: Kirigami.Theme.disabledTextColor
                wrapMode: Text.WordWrap
                Layout.fillWidth: true
            }

            Kirigami.FormLayout {
                Layout.fillWidth: true

                Controls.Label {
                    Kirigami.FormData.label: kura.tr("doctor-distro") + ":"
                    text: kura.distro
                }
                Controls.Label {
                    Kirigami.FormData.label: kura.tr("doctor-packages") + ":"
                    text: kura.packages.toLocaleString(Qt.locale(), "f", 0)
                }
                Controls.Label {
                    Kirigami.FormData.label: kura.tr("doctor-explicit") + ":"
                    text: kura.explicitCount.toLocaleString(Qt.locale(), "f", 0)
                }
                Controls.Label {
                    Kirigami.FormData.label: kura.tr("doctor-foreign") + ":"
                    text: kura.foreignCount.toLocaleString(Qt.locale(), "f", 0)
                }
                Controls.Label {
                    Kirigami.FormData.label: kura.tr("doctor-orphans") + ":"
                    text: kura.orphanCount.toLocaleString(Qt.locale(), "f", 0)
                }
            }
        }
    }
}
