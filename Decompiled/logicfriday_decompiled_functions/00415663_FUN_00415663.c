/* 00415663 FUN_00415663 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __fastcall FUN_00415663(int param_1)

{
  BOOL BVar1;
  uint unaff_retaddr;
  WINDOWPLACEMENT local_140;
  char local_114 [268];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_140.length = 0x2c;
  BVar1 = GetWindowPlacement(*(HWND *)(param_1 + 0x26c),&local_140);
  if (BVar1 != 0) {
    WritePrivateProfileStructA("Window","Placement",&local_140,0x2c,(LPCSTR)(param_1 + 0x57c));
  }
  FUN_0043ed39(local_114,&DAT_0044b960);
  WritePrivateProfileStringA("Flags","WarnMapTime",local_114,(LPCSTR)(param_1 + 0x57c));
  FUN_0043ed39(local_114,&DAT_0044b960);
  WritePrivateProfileStringA("Flags","WarnMinTime",local_114,(LPCSTR)(param_1 + 0x57c));
  FUN_0043ed39(local_114,&DAT_0044b960);
  WritePrivateProfileStringA("Flags","WarnTTExport",local_114,(LPCSTR)(param_1 + 0x57c));
  FUN_0043ed39(local_114,&DAT_0044b974);
  WritePrivateProfileStringA("Folders","szOpenDir",local_114,(LPCSTR)(param_1 + 0x57c));
  FUN_0043ed39(local_114,&DAT_0044b974);
  WritePrivateProfileStringA("Folders","szSaveDir",local_114,(LPCSTR)(param_1 + 0x57c));
  FUN_0043ed39(local_114,&DAT_0044b974);
  WritePrivateProfileStringA("Folders","szLUFcnSaveDir",local_114,(LPCSTR)(param_1 + 0x57c));
  return 1;
}
