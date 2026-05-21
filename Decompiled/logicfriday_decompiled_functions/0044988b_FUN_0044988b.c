/* 0044988b FUN_0044988b */

undefined4 __cdecl FUN_0044988b(uint *param_1,int param_2)

{
  uint *puVar1;
  int iVar2;
  int *piVar3;
  size_t sVar4;
  uint *lpName;
  uchar *puVar5;
  BOOL BVar6;
  int *piVar7;
  bool bVar8;
  undefined4 local_c;
  
  local_c = 0;
  if (param_1 == (uint *)0x0) {
    return 0xffffffff;
  }
  puVar1 = (uint *)__mbschr((uchar *)param_1,0x3d);
  if (puVar1 == (uint *)0x0) {
    return 0xffffffff;
  }
  if (param_1 == puVar1) {
    return 0xffffffff;
  }
  bVar8 = *(uchar *)((int)puVar1 + 1) == '\0';
  if (DAT_0046c700 == DAT_0046c704) {
    DAT_0046c700 = copy_environ();
  }
  if (DAT_0046c700 == (int *)0x0) {
    if ((param_2 != 0) && (DAT_0046c708 != (undefined4 *)0x0)) {
      iVar2 = FUN_004490a6();
      if (iVar2 != 0) {
        return 0xffffffff;
      }
      goto LAB_00449935;
    }
    if (!bVar8) {
      DAT_0046c700 = _malloc(4);
      if (DAT_0046c700 == (int *)0x0) {
        return 0xffffffff;
      }
      *DAT_0046c700 = 0;
      if (DAT_0046c708 == (undefined4 *)0x0) {
        DAT_0046c708 = _malloc(4);
        if (DAT_0046c708 == (undefined4 *)0x0) {
          return 0xffffffff;
        }
        *DAT_0046c708 = 0;
      }
      goto LAB_00449935;
    }
LAB_00449a39:
    local_c = 0;
  }
  else {
LAB_00449935:
    piVar3 = DAT_0046c700;
    iVar2 = findenv((uchar *)param_1);
    if ((iVar2 < 0) || (*piVar3 == 0)) {
      if (bVar8) {
        _free(param_1);
        goto LAB_00449a39;
      }
      if (iVar2 < 0) {
        iVar2 = -iVar2;
      }
      piVar3 = _realloc(piVar3,iVar2 * 4 + 8);
      if (piVar3 == (int *)0x0) {
        return 0xffffffff;
      }
      (piVar3 + iVar2)[1] = 0;
      piVar3[iVar2] = (int)param_1;
LAB_004499bf:
      DAT_0046c700 = piVar3;
    }
    else {
      piVar7 = piVar3 + iVar2;
      _free((void *)*piVar7);
      if (bVar8) {
        for (; *piVar7 != 0; piVar7 = piVar7 + 1) {
          *piVar7 = piVar7[1];
          iVar2 = iVar2 + 1;
        }
        piVar3 = _realloc(piVar3,iVar2 << 2);
        if (piVar3 != (int *)0x0) goto LAB_004499bf;
      }
      else {
        *piVar7 = (int)param_1;
      }
    }
    if (param_2 != 0) {
      sVar4 = _strlen((char *)param_1);
      lpName = _malloc(sVar4 + 2);
      if (lpName != (uint *)0x0) {
        FUN_0043ebd0(lpName,param_1);
        puVar5 = (uchar *)(((int)lpName - (int)param_1) + (int)puVar1);
        *puVar5 = '\0';
        BVar6 = SetEnvironmentVariableA((LPCSTR)lpName,(LPCSTR)(~-(uint)bVar8 & (uint)(puVar5 + 1)))
        ;
        if (BVar6 == 0) {
          local_c = 0xffffffff;
        }
        _free(lpName);
      }
    }
    if (bVar8) {
      _free(param_1);
    }
  }
  return local_c;
}
