export interface TerminalSettings {
  transparency: number;
  textColor: string;
  backgroundColor: string;
  highlightColor: string;
  fontSize: number;
  lineHeight: number;
}

export type TerminalConnection = 'connecting' | 'open' | 'closed';

export interface TerminalEntry {
  text: string;
}
