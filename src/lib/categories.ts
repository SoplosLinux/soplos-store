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

// Flathub category IDs, in the order they are shown.
export const CATEGORIES: { id: string; icon: ComponentType; background: string }[] = [
  { id: "audiovideo", icon: Film, background: "linear-gradient(135deg, #ea580c, #9a3412)" },
  { id: "development", icon: Code, background: "linear-gradient(135deg, #0369a1, #0c4a6e)" },
  { id: "education", icon: GraduationCap, background: "linear-gradient(135deg, #b45309, #78350f)" },
  { id: "game", icon: Gamepad2, background: "linear-gradient(135deg, #0f766e, #134e4a)" },
  { id: "graphics", icon: Palette, background: "linear-gradient(135deg, #7e22ce, #4c1d95)" },
  { id: "network", icon: Globe, background: "linear-gradient(135deg, #1d4ed8, #1e3a8a)" },
  { id: "office", icon: FileText, background: "linear-gradient(135deg, #be123c, #881337)" },
  { id: "science", icon: FlaskConical, background: "linear-gradient(135deg, #4d7c0f, #365314)" },
  { id: "system", icon: Monitor, background: "linear-gradient(135deg, #475569, #1e293b)" },
  { id: "utility", icon: Wrench, background: "linear-gradient(135deg, #c2410c, #7c2d12)" },
];
