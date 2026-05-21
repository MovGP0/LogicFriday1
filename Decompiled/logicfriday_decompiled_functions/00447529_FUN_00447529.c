/* 00447529 FUN_00447529 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */
/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 __cdecl FUN_00447529(UINT param_1)

{
  BYTE *pBVar1;
  byte bVar2;
  byte bVar3;
  uint uVar4;
  BOOL BVar5;
  BYTE *pBVar6;
  int iVar7;
  int extraout_ECX;
  undefined4 extraout_ECX_00;
  int iVar8;
  byte *pbVar9;
  byte *pbVar10;
  undefined4 *puVar11;
  uint unaff_retaddr;
  _cpinfo local_20;
  uint local_c;
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if (param_1 != 0) {
    iVar8 = 0;
    uVar4 = 0;
    do {
      if (*(UINT *)((int)&DAT_00452448 + uVar4) == param_1) {
        puVar11 = (undefined4 *)&DAT_0046ca00;
        for (iVar7 = 0x40; iVar7 != 0; iVar7 = iVar7 + -1) {
          *puVar11 = 0;
          puVar11 = puVar11 + 1;
        }
        local_c = 0;
        *(undefined1 *)puVar11 = 0;
        pbVar9 = (byte *)(iVar8 * 0x30 + 0x452458);
        do {
          bVar3 = *pbVar9;
          pbVar10 = pbVar9;
          while ((bVar3 != 0 && (bVar2 = pbVar10[1], bVar2 != 0))) {
            uVar4 = (uint)bVar3;
            if (uVar4 <= bVar2) {
              bVar3 = (&DAT_00452440)[local_c];
              do {
                (&DAT_0046ca01)[uVar4] = (&DAT_0046ca01)[uVar4] | bVar3;
                uVar4 = uVar4 + 1;
              } while (uVar4 <= bVar2);
            }
            pbVar10 = pbVar10 + 2;
            bVar3 = *pbVar10;
          }
          local_c = local_c + 1;
          pbVar9 = pbVar9 + 8;
        } while (local_c < 4);
        DAT_0046cb04 = param_1;
        DAT_0046c9fc = 1;
        DAT_0046c9f4 = FUN_004472d0();
        _DAT_0046cb10 = *(undefined4 *)(&DAT_0045244c + extraout_ECX);
        DAT_0046cb14 = *(undefined4 *)(&DAT_00452450 + extraout_ECX);
        DAT_0046cb18 = *(undefined4 *)(&DAT_00452454 + extraout_ECX);
        goto LAB_004476a3;
      }
      uVar4 = uVar4 + 0x30;
      iVar8 = iVar8 + 1;
    } while (uVar4 < 0xf0);
    BVar5 = GetCPInfo(param_1,&local_20);
    if (BVar5 == 1) {
      puVar11 = (undefined4 *)&DAT_0046ca00;
      for (iVar8 = 0x40; iVar8 != 0; iVar8 = iVar8 + -1) {
        *puVar11 = 0;
        puVar11 = puVar11 + 1;
      }
      *(undefined1 *)puVar11 = 0;
      DAT_0046cb04 = param_1;
      DAT_0046c9f4 = 0;
      if (local_20.MaxCharSize < 2) {
        DAT_0046c9fc = 0;
      }
      else {
        if (local_20.LeadByte[0] != '\0') {
          pBVar6 = local_20.LeadByte + 1;
          do {
            bVar3 = *pBVar6;
            if (bVar3 == 0) break;
            for (uVar4 = (uint)pBVar6[-1]; uVar4 <= bVar3; uVar4 = uVar4 + 1) {
              (&DAT_0046ca01)[uVar4] = (&DAT_0046ca01)[uVar4] | 4;
            }
            pBVar1 = pBVar6 + 1;
            pBVar6 = pBVar6 + 2;
          } while (*pBVar1 != 0);
        }
        uVar4 = 1;
        do {
          (&DAT_0046ca01)[uVar4] = (&DAT_0046ca01)[uVar4] | 8;
          uVar4 = uVar4 + 1;
        } while (uVar4 < 0xff);
        DAT_0046c9f4 = FUN_004472d0();
        DAT_0046c9fc = extraout_ECX_00;
      }
      _DAT_0046cb10 = 0;
      DAT_0046cb14 = 0;
      DAT_0046cb18 = 0;
      goto LAB_004476a3;
    }
    if (DAT_0046c9b0 == 0) {
      return 0xffffffff;
    }
  }
  setSBCS();
LAB_004476a3:
  FUN_00447328();
  return 0;
}
