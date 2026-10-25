// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

// Pages scroll inside the window; the window never resizes itself (docs/ui.md).

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami
import org.kura.app

Kirigami.ApplicationWindow {
    id: root

    title: "kura"
    width: Kirigami.Units.gridUnit * 50
    height: Kirigami.Units.gridUnit * 36
    minimumWidth: Kirigami.Units.gridUnit * 24
    minimumHeight: Kirigami.Units.gridUnit * 24

    Kura {
        id: kura
        Component.onCompleted: refresh()
    }

    function show(page) {
        pageStack.clear()
        pageStack.push(page)
    }

    globalDrawer: Kirigami.GlobalDrawer {
        title: "kura"
        titleIcon: "kura"
        modal: !root.wideScreen
        collapsible: false
        isMenu: false
        actions: [
            Kirigami.Action {
                text: kura.tr("nav-overview")
                icon.name: "go-home"
                onTriggered: root.show(overviewPage)
            },
            Kirigami.Action {
                text: kura.tr("nav-updates")
                icon.name: "update-none"
                onTriggered: { root.show(updatesPage); kura.prepareUpdate() }
            },
            Kirigami.Action {
                text: kura.tr("nav-history")
                icon.name: "view-history"
                onTriggered: { kura.refresh(); root.show(historyPage) }
            },
            Kirigami.Action {
                text: kura.tr("nav-settings")
                icon.name: "settings-configure"
                onTriggered: root.show(settingsPage)
            }
        ]
    }

    pageStack.initialPage: overviewPage

    Component {
        id: overviewPage

        Kirigami.ScrollablePage {
            title: kura.tr("nav-overview")

            ColumnLayout {
                spacing: Kirigami.Units.largeSpacing

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: kura.error.length > 0
                    type: Kirigami.MessageType.Error
                    text: kura.error
                }

                // One status with one main action.
                Kirigami.AbstractCard {
                    Layout.fillWidth: true
                    contentItem: RowLayout {
                        spacing: Kirigami.Units.largeSpacing
                        Kirigami.Icon {
                            source: kura.state === 4 ? "security-low" : kura.state === 3 ? "data-warning" : kura.state === 2 ? "update-medium" : "data-success"
                            Layout.preferredWidth: Kirigami.Units.iconSizes.large
                            Layout.preferredHeight: Kirigami.Units.iconSizes.large
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Kirigami.Heading {
                                text: kura.statusText
                                level: 2
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                            Controls.Label {
                                text: kura.lastCheck
                                visible: text.length > 0
                                color: Kirigami.Theme.disabledTextColor
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }
                        Controls.Button {
                            text: kura.state >= 2 ? kura.tr("gui-review") : kura.tr("gui-check-now")
                            icon.name: "update-none"
                            highlighted: kura.state >= 2
                            onClicked: { root.show(updatesPage); kura.prepareUpdate() }
                        }
                    }
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

    Component {
        id: updatesPage

        Kirigami.ScrollablePage {
            title: kura.previewTitle.length > 0 ? kura.previewTitle : kura.tr("nav-updates")

            property var groups: JSON.parse(kura.previewJson)
            property var news: JSON.parse(kura.newsJson)

            footer: Controls.ToolBar {
                visible: kura.canInstall && !kura.busy
                contentItem: RowLayout {
                    Item { Layout.fillWidth: true }
                    Controls.Button {
                        text: kura.tr("gui-cancel")
                        onClicked: root.show(overviewPage)
                    }
                    Controls.Button {
                        readonly property bool restoring: kura.previewTitle !== kura.tr("nav-updates")
                        text: restoring ? kura.tr("gui-go-back") : news.length > 0 ? kura.tr("gui-install-news") : kura.tr("gui-install")
                        icon.name: "run-install"
                        highlighted: true
                        onClicked: kura.installUpdate()
                    }
                }
            }

            ColumnLayout {
                spacing: Kirigami.Units.largeSpacing

                RowLayout {
                    visible: kura.busy || kura.progressText.length > 0
                    Layout.fillWidth: true
                    Controls.BusyIndicator {
                        running: kura.busy
                        visible: kura.busy
                    }
                    Kirigami.Heading {
                        text: kura.progressText
                        level: 3
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                }

                Kirigami.InlineMessage {
                    Layout.fillWidth: true
                    visible: kura.message.length > 0
                    type: Kirigami.MessageType.Information
                    text: kura.message
                }

                Repeater {
                    model: news
                    Kirigami.InlineMessage {
                        Layout.fillWidth: true
                        visible: true
                        type: Kirigami.MessageType.Warning
                        text: modelData.title
                        actions: [
                            Kirigami.Action {
                                text: kura.tr("gui-read-news")
                                icon.name: "internet-services"
                                onTriggered: Qt.openUrlExternally(modelData.link)
                            }
                        ]
                    }
                }

                Controls.Label {
                    visible: groups.length > 0 && !kura.busy
                    text: kura.tr("gui-updates-intro")
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                Repeater {
                    model: kura.busy ? [] : groups
                    ColumnLayout {
                        Layout.fillWidth: true
                        Kirigami.Heading {
                            text: modelData.title
                            level: 4
                        }
                        Repeater {
                            model: modelData.items
                            RowLayout {
                                Layout.fillWidth: true
                                Controls.Label {
                                    text: modelData.name
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                Controls.Label {
                                    text: modelData.detail
                                    font.family: "monospace"
                                    color: Kirigami.Theme.disabledTextColor
                                }
                            }
                        }
                    }
                }

                Controls.Label {
                    visible: groups.length > 0 && !kura.busy
                    text: kura.summaryText
                    color: Kirigami.Theme.disabledTextColor
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }
    }

    Component {
        id: historyPage

        Kirigami.ScrollablePage {
            title: kura.tr("nav-history")
            property var records: JSON.parse(kura.historyJson)

            ListView {
                model: records
                delegate: Controls.ItemDelegate {
                    width: ListView.view.width
                    contentItem: RowLayout {
                        Controls.Label {
                            text: "#" + modelData.id
                            color: Kirigami.Theme.disabledTextColor
                        }
                        Controls.Label {
                            text: modelData.text
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                        }
                        Controls.Label {
                            text: modelData.when
                            color: Kirigami.Theme.disabledTextColor
                        }
                        Controls.Button {
                            visible: index > 0
                            text: kura.tr("gui-go-back")
                            icon.name: "edit-undo"
                            onClicked: { root.show(updatesPage); kura.prepareRestore(modelData.id) }
                        }
                    }
                }
                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.gridUnit * 4
                    visible: records.length === 0
                    text: kura.tr("history-empty")
                }
            }
        }
    }

    Component {
        id: settingsPage

        Kirigami.ScrollablePage {
            title: kura.tr("nav-settings")

            Kirigami.FormLayout {
                Controls.Switch {
                    Kirigami.FormData.label: kura.tr("gui-tray-mono") + ":"
                    checked: kura.monochrome
                    onToggled: kura.setTrayMonochrome(checked)
                }
                Controls.Label {
                    text: kura.tr("gui-tray-mono-hint")
                    color: Kirigami.Theme.disabledTextColor
                    wrapMode: Text.WordWrap
                }
            }
        }
    }
}
