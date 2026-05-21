/* 0040a1fb FUN_0040a1fb */

void __cdecl FUN_0040a1fb(int *param_1)

{
  HWND hDlg;
  
  do {
    Sleep(0);
    hDlg = FindWindowA("#32770","Variable Name");
  } while (hDlg == (HWND)0x0);
  if (*param_1 == 0x439) {
    SetDlgItemTextA(hDlg,0x43e,"X");
  }
  else {
    SetDlgItemTextA(hDlg,0x43e,(LPCSTR)param_1[1]);
  }
  SendMessageA(hDlg,0x111,1,0);
  FUN_0043ea5f();
  return;
}
