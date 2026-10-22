import type { ComponentType } from "svelte";
import { Film, Code, GraduationCap, Gamepad2, Palette, Globe, FileText, FlaskConical, Monitor, Wrench } from "lucide-svelte";

// AppStream subcategories offered inside each category page. Flathub and
// Debian both use these names; a page lists only those with apps in it.
export const SUBCATEGORIES: Record<string, string[]> = {
  audiovideo: ["Audio", "Video", "Player", "Music", "AudioVideoEditing", "Recorder"],
  development: ["IDE", "Debugger", "RevisionControl", "WebDevelopment", "GUIDesigner"],
  education: ["Languages", "Math", "Astronomy", "Geography", "ComputerScience"],
  game: ["ActionGame", "AdventureGame", "ArcadeGame", "BoardGame", "LogicGame", "RolePlaying", "Simulation", "StrategyGame", "Emulator"],
  graphics: ["2DGraphics", "3DGraphics", "VectorGraphics", "RasterGraphics", "Viewer", "Photography", "Scanning"],
  network: ["WebBrowser", "Email", "Chat", "InstantMessaging", "FileTransfer", "P2P", "RemoteAccess", "News"],
  office: ["WordProcessor", "Spreadsheet", "Presentation", "Calendar", "Database", "Finance"],
  science: ["Astronomy", "Math", "Chemistry", "Physics", "Biology", "Electronics", "Engineering"],
  system: ["Monitor", "Security", "PackageManager", "FileManager", "TerminalEmulator", "Settings"],
  utility: ["Archiving", "TextEditor", "Calculator", "Clock", "Accessibility"],
};

// Flathub category IDs, in the order they are shown, each with the color
// ("r g b") of its card.
export const CATEGORIES: { id: string; icon: ComponentType; color: string }[] = [
  { id: "audiovideo", icon: Film, color: "249 115 22" },
  { id: "development", icon: Code, color: "14 165 233" },
  { id: "education", icon: GraduationCap, color: "245 158 11" },
  { id: "game", icon: Gamepad2, color: "20 184 166" },
  { id: "graphics", icon: Palette, color: "168 85 247" },
  { id: "network", icon: Globe, color: "59 130 246" },
  { id: "office", icon: FileText, color: "244 63 94" },
  { id: "science", icon: FlaskConical, color: "132 204 22" },
  { id: "system", icon: Monitor, color: "148 163 184" },
  { id: "utility", icon: Wrench, color: "251 146 60" },
];
