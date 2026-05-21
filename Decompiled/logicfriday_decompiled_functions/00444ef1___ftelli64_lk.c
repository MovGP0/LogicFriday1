/* 00444ef1 __ftelli64_lk */

/* Library Function - Single Match
    __ftelli64_lk
   
   Library: Visual Studio 2003 Release */

ulonglong __cdecl __ftelli64_lk(uint *param_1)

{
  uint _FileHandle;
  byte bVar1;
  ulonglong uVar2;
  int *piVar3;
  uint *puVar4;
  char *pcVar5;
  uint *puVar6;
  char *pcVar7;
  char *pcVar8;
  uint uVar9;
  int iVar10;
  int unaff_EDI;
  longlong lVar11;
  undefined8 local_14;
  uint local_8;
  
  puVar6 = param_1;
  _FileHandle = param_1[4];
  if ((int)param_1[1] < 0) {
    param_1[1] = 0;
  }
  local_14 = __lseeki64(_FileHandle,0x100000000,unaff_EDI);
  uVar9 = (uint)((ulonglong)local_14 >> 0x20);
  if ((uVar9 != 0 && -1 < local_14) || (-1 < local_14)) {
    if ((param_1[3] & 0x108) == 0) {
      return local_14 - (int)param_1[1];
    }
    pcVar5 = (char *)*param_1;
    pcVar8 = (char *)param_1[2];
    local_8 = (int)pcVar5 - (int)pcVar8;
    if ((param_1[3] & 3) == 0) {
      if (-1 < (char)param_1[3]) {
        piVar3 = FUN_00441a24();
        *piVar3 = 0x16;
        goto LAB_00444fae;
      }
    }
    else if (((*(byte *)((&DAT_0046cc40)[(int)_FileHandle >> 5] + 4 + (_FileHandle & 0x1f) * 0x24) &
              0x80) != 0) && (pcVar7 = pcVar8, pcVar8 < pcVar5)) {
      do {
        if (*pcVar7 == '\n') {
          local_8 = local_8 + 1;
        }
        pcVar7 = pcVar7 + 1;
      } while (pcVar7 < (char *)*param_1);
    }
    if (local_14 == 0) {
      uVar2 = (ulonglong)local_8;
    }
    else {
      if ((param_1[3] & 1) != 0) {
        if (param_1[1] == 0) {
          local_8 = 0;
        }
        else {
          puVar4 = (uint *)(pcVar5 + (param_1[1] - (int)pcVar8));
          iVar10 = (_FileHandle & 0x1f) * 0x24;
          if ((*(byte *)(iVar10 + 4 + (&DAT_0046cc40)[(int)_FileHandle >> 5]) & 0x80) != 0) {
            lVar11 = __lseeki64(_FileHandle,0x200000000,unaff_EDI);
            if (lVar11 == local_14) {
              pcVar5 = (char *)param_1[2];
              pcVar8 = (char *)((int)puVar4 + (int)pcVar5);
              param_1 = puVar4;
              for (; pcVar5 < pcVar8; pcVar5 = pcVar5 + 1) {
                if (*pcVar5 == '\n') {
                  param_1 = (uint *)((int)param_1 + 1);
                }
              }
              bVar1 = *(byte *)((int)puVar6 + 0xd) & 0x20;
            }
            else {
              __lseeki64(_FileHandle,(ulonglong)uVar9,unaff_EDI);
              puVar6 = (uint *)0x200;
              if ((((uint *)0x200 < puVar4) || ((param_1[3] & 8) == 0)) ||
                 ((param_1[3] & 0x400) != 0)) {
                puVar6 = (uint *)param_1[6];
              }
              bVar1 = *(byte *)(iVar10 + 4 + (&DAT_0046cc40)[(int)_FileHandle >> 5]) & 4;
              param_1 = puVar6;
            }
            puVar4 = param_1;
            if (bVar1 != 0) {
              puVar4 = (uint *)((int)param_1 + 1);
            }
          }
          param_1 = puVar4;
          local_14 = CONCAT44(uVar9 - ((uint *)local_14 < param_1),
                              (int)(uint *)local_14 - (int)param_1);
        }
      }
      uVar2 = CONCAT44(local_14._4_4_ + (uint)CARRY4(local_8,(uint)local_14),
                       local_8 + (uint)local_14);
    }
  }
  else {
LAB_00444fae:
    uVar2 = 0xffffffffffffffff;
  }
  return uVar2;
}
