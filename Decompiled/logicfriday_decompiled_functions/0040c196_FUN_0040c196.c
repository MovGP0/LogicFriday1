/* 0040c196 FUN_0040c196 */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __cdecl FUN_0040c196(undefined4 *param_1)

{
  DWORD DVar1;
  BOOL BVar2;
  int iVar3;
  undefined4 *puVar4;
  uint unaff_retaddr;
  _STARTUPINFOA local_13ac;
  DWORD local_1360;
  char local_135c;
  undefined4 local_135b;
  _SECURITY_ATTRIBUTES local_1254;
  HANDLE local_1248;
  char local_1244;
  undefined4 local_1243;
  _PROCESS_INFORMATION local_1138;
  int local_1128;
  char local_1124;
  undefined4 local_1123;
  undefined1 local_101c;
  undefined4 local_101b;
  uint local_18;
  undefined4 *local_14;
  HANDLE local_10;
  HANDLE local_c;
  DWORD local_8;
  
  local_18 = DAT_00451a00 ^ unaff_retaddr;
  local_14 = param_1;
  local_1248 = (HANDLE)0x0;
  local_8 = 1;
  local_1124 = '\0';
  puVar4 = &local_1123;
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  local_135c = '\0';
  puVar4 = &local_135b;
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  local_1244 = '\0';
  puVar4 = &local_1243;
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  local_101c = 0;
  puVar4 = &local_101b;
  for (iVar3 = 0x3ff; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  _memset(&local_13ac,0,0x44);
  local_13ac.cb = 0x44;
  local_1254.bInheritHandle = 1;
  local_1254.lpSecurityDescriptor = (LPVOID)0x0;
  local_1254.nLength = 0xc;
  local_10 = (HANDLE)local_14[0x42];
  if (local_14[1] == 0) {
    FUN_0043ed39(&local_1124,(byte *)"%s\\espresso\\espresso.exe");
    FUN_0043ed39(&local_1244,(byte *)"%s\\espresso");
  }
  else {
    FUN_0043ed39(&local_1124,(byte *)"%s\\misii\\misii.exe");
    FUN_0043ed39(&local_1244,(byte *)"%s\\misii");
  }
  FUN_0043ed39(&local_135c,(byte *)"%s\\minout.dat");
  local_1248 = CreateFileA(&local_135c,0xc0000000,3,&local_1254,2,0x80,(HANDLE)0x0);
  if (local_1248 == (HANDLE)0xffffffff) {
    DVar1 = GetLastError();
    local_14[0x43] = DVar1;
    FUN_0040a274(DAT_00452aac,0x2f000a);
    if (local_14[1] == 0) {
      PostMessageA((HWND)*local_14,0x111,0x18004,0);
    }
    else {
      PostMessageA((HWND)*local_14,0x111,0x18006,0);
    }
    FUN_0043ea5f();
  }
  local_13ac.hStdOutput = local_1248;
  local_13ac.hStdInput = GetStdHandle(0xfffffff6);
  local_13ac.hStdError = local_1248;
  local_13ac.dwFlags = 0x101;
  local_13ac.wShowWindow = 0;
  BVar2 = CreateProcessA(&local_1124,(LPSTR)(local_14 + 2),(LPSECURITY_ATTRIBUTES)0x0,
                         (LPSECURITY_ATTRIBUTES)0x0,1,0x200,(LPVOID)0x0,&local_1244,&local_13ac,
                         &local_1138);
  if (BVar2 == 0) {
    local_1360 = GetLastError();
  }
  else {
    local_c = local_1138.hProcess;
    CloseHandle(local_1138.hThread);
    DVar1 = WaitForMultipleObjects(2,&local_10,0,0xffffffff);
    if (DVar1 == 0) {
      if ((DAT_00452efc != 0) && (DAT_00452efc != 1)) {
        (*DAT_00452a94)(local_1138.dwProcessId);
      }
      GenerateConsoleCtrlEvent(1,local_1138.dwProcessId);
      WaitForSingleObject(local_1138.hProcess,0xffffffff);
      local_1128 = 1;
      if ((DAT_00452efc != 0) && (DAT_00452efc != 1)) {
        FreeConsole();
      }
    }
    else if (DVar1 == 1) {
      GetExitCodeProcess(local_1138.hProcess,&local_8);
      local_1128 = 0;
    }
    else if (DVar1 == 0xffffffff) {
      local_1128 = 2;
    }
    CloseHandle(local_1138.hProcess);
    CloseHandle(local_1248);
  }
  if (local_14[1] == 0) {
    PostMessageA((HWND)*local_14,0x111,local_1128 << 0x10 | 0x8004,0);
  }
  else {
    PostMessageA((HWND)*local_14,0x111,local_1128 << 0x10 | 0x8006,0);
  }
  FUN_0043ea5f();
  return;
}
