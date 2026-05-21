/* 004484ea FUN_004484ea */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __cdecl FUN_004484ea(uint param_1,int param_2)

{
  DWORD DVar1;
  uint uVar2;
  int iVar3;
  ulong *puVar4;
  int *piVar5;
  HANDLE hFile;
  BOOL BVar6;
  int iVar7;
  uint uVar8;
  uint unaff_retaddr;
  char local_1010 [4096];
  int local_10;
  DWORD local_c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  iVar7 = 0;
  local_c = __lseek_lk(param_1,0,1);
  if ((local_c == 0xffffffff) || (DVar1 = __lseek_lk(param_1,0,2), DVar1 == 0xffffffff)) {
    iVar7 = -1;
  }
  else {
    uVar8 = param_2 - DVar1;
    if ((int)uVar8 < 1) {
      if ((int)uVar8 < 0) {
        __lseek_lk(param_1,param_2,0);
        hFile = (HANDLE)__get_osfhandle(param_1);
        BVar6 = SetEndOfFile(hFile);
        iVar7 = (BVar6 != 0) - 1;
        if (iVar7 == -1) {
          piVar5 = FUN_00441a24();
          *piVar5 = 0xd;
          puVar4 = FUN_00441a2d();
          DVar1 = GetLastError();
          *puVar4 = DVar1;
        }
      }
    }
    else {
      _memset(local_1010,0,0x1000);
      local_10 = FUN_00449111(param_1,0x8000);
      do {
        uVar2 = 0x1000;
        if ((int)uVar8 < 0x1000) {
          uVar2 = uVar8;
        }
        iVar3 = FUN_0044410a(param_1,local_1010,uVar2);
        if (iVar3 == -1) {
          puVar4 = FUN_00441a2d();
          if (*puVar4 == 5) {
            piVar5 = FUN_00441a24();
            *piVar5 = 0xd;
          }
          iVar7 = -1;
          break;
        }
        uVar8 = uVar8 - iVar3;
      } while (0 < (int)uVar8);
      FUN_00449111(param_1,local_10);
    }
    __lseek_lk(param_1,local_c,0);
  }
  return iVar7;
}
