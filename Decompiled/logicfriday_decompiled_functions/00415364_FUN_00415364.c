/* 00415364 FUN_00415364 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __fastcall FUN_00415364(int param_1)

{
  BOOL BVar1;
  int iVar2;
  UINT UVar3;
  uint unaff_retaddr;
  undefined4 local_144;
  WINDOWPLACEMENT local_140;
  uint local_114 [67];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_144 = 1;
  BVar1 = GetPrivateProfileStructA("Window","Placement",&local_140,0x2c,(LPCSTR)(param_1 + 0x57c));
  if (BVar1 == 0) {
    ShowWindow(*(HWND *)(param_1 + 0x26c),1);
  }
  else {
    local_140.length = 0x2c;
    if (local_140.showCmd == 2) {
      local_140.showCmd = 1;
    }
    else if (local_140.showCmd == 3) {
      *(LONG *)(param_1 + 0x208) = local_140.rcNormalPosition.left;
      *(LONG *)(param_1 + 0x20c) = local_140.rcNormalPosition.top;
      *(LONG *)(param_1 + 0x210) = local_140.rcNormalPosition.right;
      *(LONG *)(param_1 + 0x214) = local_140.rcNormalPosition.bottom;
      DAT_00452ef8 = 1;
    }
    SetWindowPlacement(*(HWND *)(param_1 + 0x26c),&local_140);
  }
  GetPrivateProfileStringA
            ("Folders","szOpenDir","none",(LPSTR)local_114,0x104,(LPCSTR)(param_1 + 0x57c));
  iVar2 = __strnicmp((char *)local_114,"c:\\",3);
  if (iVar2 == 0) {
    FUN_0043ebd0((uint *)(param_1 + 0x270),local_114);
  }
  GetPrivateProfileStringA
            ("Folders","szSaveDir","none",(LPSTR)local_114,0x104,(LPCSTR)(param_1 + 0x57c));
  iVar2 = __strnicmp((char *)local_114,"c:\\",3);
  if (iVar2 == 0) {
    FUN_0043ebd0((uint *)(param_1 + 0x374),local_114);
  }
  GetPrivateProfileStringA
            ("Folders","szLUFcnSaveDir","none",(LPSTR)local_114,0x104,(LPCSTR)(param_1 + 0x57c));
  iVar2 = __strnicmp((char *)local_114,"c:\\",3);
  if (iVar2 == 0) {
    FUN_0043ebd0((uint *)(param_1 + 0x478),local_114);
  }
  UVar3 = GetPrivateProfileIntA("Flags","WarnMapTime",-1,(LPCSTR)(param_1 + 0x57c));
  if ((UVar3 != 0) && (UVar3 != 1)) {
    local_144 = 0;
    UVar3 = DAT_00452ee0;
  }
  DAT_00452ee0 = UVar3;
  UVar3 = GetPrivateProfileIntA("Flags","WarnMinTime",-1,(LPCSTR)(param_1 + 0x57c));
  if ((UVar3 != 0) && (UVar3 != 1)) {
    local_144 = 0;
    UVar3 = DAT_00452ee4;
  }
  DAT_00452ee4 = UVar3;
  UVar3 = GetPrivateProfileIntA("Flags","WarnTTExport",-1,(LPCSTR)(param_1 + 0x57c));
  if ((UVar3 != 0) && (UVar3 != 1)) {
    local_144 = 0;
    UVar3 = DAT_00452ee8;
  }
  DAT_00452ee8 = UVar3;
  return local_144;
}
