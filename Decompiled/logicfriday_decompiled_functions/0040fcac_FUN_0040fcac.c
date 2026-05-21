/* 0040fcac FUN_0040fcac */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

bool __thiscall FUN_0040fcac(void *this,HWND param_1,uint param_2)

{
  BOOL BVar1;
  HANDLE pvVar2;
  size_t sVar3;
  char *pcVar4;
  bool bVar5;
  uint unaff_retaddr;
  _WIN32_FIND_DATAA local_574;
  undefined4 local_434;
  LPSTR *local_430;
  uint local_42c [66];
  CHAR local_324 [524];
  DWORD local_118;
  char local_114 [268];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  local_430 = (LPSTR *)0x0;
  local_434 = 1;
  *(HWND *)((int)this + 0x26c) = param_1;
  *(uint *)((int)this + 0x218) = param_2;
  GetModuleFileNameA((HMODULE)0x0,(LPSTR)local_42c,0x104);
  FUN_00415864((char *)local_42c,this);
  BVar1 = SHGetSpecialFolderPathA(param_1,(LPSTR)local_42c,0x1a,1);
  if (BVar1 != 0) {
    FUN_0043ebe0(local_42c,(uint *)"\\Logic Friday");
    pvVar2 = FindFirstFileA((LPCSTR)local_42c,&local_574);
    if (pvVar2 == (HANDLE)0xffffffff) {
      BVar1 = CreateDirectoryA((LPCSTR)local_42c,(LPSECURITY_ATTRIBUTES)0x0);
      if (BVar1 == 0) {
        FUN_0040a274(param_1,0x2e0009);
        SendMessageA(param_1,0x10,0,0);
        return true;
      }
    }
    else {
      FindClose(pvVar2);
    }
  }
  FUN_0043ed39((char *)((int)this + 0x57c),(byte *)"%s\\lf.ini");
  local_118 = GetShortPathNameA((LPCSTR)local_42c,(LPSTR)((int)this + 0x104),0x104);
  sVar3 = _strlen((char *)((int)this + 0x104));
  if ((((local_118 == 0) || (sVar3 < 2)) || (0x46 < sVar3)) ||
     (pcVar4 = _strchr((char *)((int)this + 0x104),0x20), pcVar4 != (char *)0x0)) {
    FUN_0043ebd0((uint *)((int)this + 0x104),(uint *)&DAT_0044ad26);
    _strncpy((char *)((int)this + 0x104),this,3);
    *(undefined1 *)((int)this + 0x107) = 0;
    FUN_0043ebe0((uint *)((int)this + 0x104),(uint *)"logfriday_TEMP");
    pvVar2 = FindFirstFileA((LPCSTR)((int)this + 0x104),&local_574);
    if (pvVar2 == (HANDLE)0xffffffff) {
      BVar1 = CreateDirectoryA((LPCSTR)((int)this + 0x104),(LPSECURITY_ATTRIBUTES)0x0);
      if (BVar1 == 0) {
        FUN_0040a274(param_1,0x2e0009);
        SendMessageA(param_1,0x10,0,0);
        return true;
      }
    }
    else {
      FindClose(pvVar2);
    }
    *(undefined4 *)((int)this + 0xda4) = 1;
  }
  FUN_0043ed39((char *)((int)this + 0x888),(byte *)"%s\\minout.dat");
  FUN_0043ed39((char *)((int)this + 0x784),(byte *)"%s\\minin.dat");
  FUN_0043ed39((char *)((int)this + 0xa90),(byte *)"%s\\checkout.dat");
  FUN_0043ed39((char *)((int)this + 0x98c),(byte *)"%s\\checkin.dat");
  FUN_0043ed39((char *)((int)this + 0xc98),(byte *)"%s\\user.genlib");
  BVar1 = SHGetSpecialFolderPathA(param_1,(LPSTR)((int)this + 0x270),5,1);
  if (BVar1 == 0) {
    FUN_0043ebd0((uint *)((int)this + 0x270),(uint *)&DAT_0044ba34);
  }
  FUN_0043ebd0((uint *)((int)this + 0x374),(uint *)((int)this + 0x270));
  FUN_0043ebd0((uint *)((int)this + 0x478),(uint *)((int)this + 0x270));
  *(undefined4 *)((int)this + 0x21c) = 0x4c;
  *(HWND *)((int)this + 0x220) = param_1;
  *(undefined4 *)((int)this + 0x224) = 0;
  *(undefined4 *)((int)this + 0x228) = 0;
  *(undefined4 *)((int)this + 0x22c) = 0;
  *(undefined4 *)((int)this + 0x230) = 0;
  *(undefined4 *)((int)this + 0x234) = 0;
  *(undefined4 *)((int)this + 0x238) = 0;
  *(undefined4 *)((int)this + 0x23c) = 0x104;
  *(undefined4 *)((int)this + 0x240) = 0;
  *(undefined4 *)((int)this + 0x244) = 0x104;
  *(undefined4 *)((int)this + 0x248) = 0;
  *(undefined4 *)((int)this + 0x24c) = 0;
  *(undefined4 *)((int)this + 0x250) = 0;
  *(undefined2 *)((int)this + 0x254) = 0;
  *(undefined2 *)((int)this + 0x256) = 0;
  *(undefined **)((int)this + 600) = &DAT_0044ba30;
  *(undefined4 *)((int)this + 0x25c) = 0;
  *(undefined4 *)((int)this + 0x260) = 0;
  *(undefined4 *)((int)this + 0x264) = 0;
  FUN_0043ed39(local_114,(byte *)"%s\\espresso");
  FUN_0043ebd0(local_42c,(uint *)"espresso.exe");
  local_118 = SearchPathA(local_114,(LPCSTR)local_42c,"",0x104,local_324,local_430);
  if (local_118 != 0) {
    FUN_0043ed39(local_114,(byte *)"%s\\misii");
    FUN_0043ebd0(local_42c,(uint *)"misii.exe");
    local_118 = SearchPathA(local_114,(LPCSTR)local_42c,"",0x104,local_324,local_430);
  }
  if (local_118 != 0) {
    FUN_0043ed39(local_114,(byte *)"%s\\misii\\lib");
    FUN_0043ebd0(local_42c,(uint *)"script.txt");
    local_118 = SearchPathA(local_114,(LPCSTR)local_42c,"",0x104,local_324,local_430);
  }
  bVar5 = local_118 != 0;
  if (!bVar5) {
    FUN_0043ed39(local_324,
                 (byte *)
                 "Logic Friday cannot continue because an essential file\ncould not be found. It may have been moved or deleted.\n\n%s\\%s"
                );
    MessageBoxA(*(HWND *)((int)this + 0x26c),local_324,"File Error",0);
  }
  return bVar5;
}
