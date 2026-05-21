/* 0040c55b FUN_0040c55b */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __cdecl FUN_0040c55b(undefined4 *param_1)

{
  undefined4 uVar1;
  BOOL BVar2;
  int iVar3;
  undefined4 *puVar4;
  uint unaff_retaddr;
  _STARTUPINFOA local_139c;
  char local_134c;
  undefined4 local_134b;
  _SECURITY_ATTRIBUTES local_1244;
  HANDLE local_1238;
  undefined1 local_1234;
  undefined4 local_1233;
  _PROCESS_INFORMATION local_112c;
  char local_111c;
  undefined4 local_111b;
  undefined1 local_1014;
  undefined4 local_1013;
  uint local_10;
  undefined4 *local_c;
  undefined4 local_8;
  
  local_10 = DAT_00451a00 ^ unaff_retaddr;
  local_c = param_1;
  local_1238 = (HANDLE)0x0;
  local_8 = 1;
  local_111c = '\0';
  puVar4 = &local_111b;
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  local_134c = '\0';
  puVar4 = &local_134b;
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  local_1234 = 0;
  puVar4 = &local_1233;
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  local_1014 = 0;
  puVar4 = &local_1013;
  for (iVar3 = 0x3ff; iVar3 != 0; iVar3 = iVar3 + -1) {
    *puVar4 = 0;
    puVar4 = puVar4 + 1;
  }
  *(undefined2 *)puVar4 = 0;
  *(undefined1 *)((int)puVar4 + 2) = 0;
  _memset(&local_139c,0,0x44);
  local_139c.cb = 0x44;
  local_1244.bInheritHandle = 1;
  local_1244.lpSecurityDescriptor = (LPVOID)0x0;
  local_1244.nLength = 0xc;
  if (local_c[1] == 0) {
    FUN_0043ed39(&local_111c,(byte *)"%s\\espresso\\espresso.exe");
  }
  else {
    FUN_0043ed39(&local_111c,(byte *)"%s\\misii\\misii.exe");
  }
  FUN_0043ed39(&local_134c,(byte *)"%s\\minout.dat");
  local_139c.hStdOutput = CreateFileA(&local_134c,0xc0000000,3,&local_1244,2,0x80,(HANDLE)0x0);
  local_1238 = local_139c.hStdOutput;
  if (local_139c.hStdOutput == (HANDLE)0xffffffff) {
    GetLastError();
    uVar1 = 1;
  }
  else {
    local_139c.hStdInput = GetStdHandle(0xfffffff6);
    local_139c.hStdError = local_1238;
    local_139c.dwFlags = 0x101;
    local_139c.wShowWindow = 0;
    BVar2 = CreateProcessA(&local_111c,(LPSTR)(local_c + 2),(LPSECURITY_ATTRIBUTES)0x0,
                           (LPSECURITY_ATTRIBUTES)0x0,1,0x200,(LPVOID)0x0,&DAT_00453044,&local_139c,
                           &local_112c);
    if (BVar2 == 0) {
      GetLastError();
      uVar1 = 1;
    }
    else {
      CloseHandle(local_112c.hThread);
      WaitForSingleObject(local_112c.hProcess,3600000);
      CloseHandle(local_112c.hProcess);
      CloseHandle(local_1238);
      if (local_c[1] == 0) {
        SendMessageA((HWND)*local_c,0x111,0x8004,0);
      }
      else {
        SendMessageA((HWND)*local_c,0x111,0x8006,0);
      }
      uVar1 = 0;
    }
  }
  return uVar1;
}
